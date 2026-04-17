use std::sync::Arc;

use async_trait::async_trait;

use crate::runtime::{
    RuntimeTool, ToolContent, ToolDefinitionSpec, ToolDenied, ToolExecuteError,
    ToolExecutionContext, ToolExecutionOutcome, ToolFailure, ToolProgressReporter,
    ToolResultMetadata, ToolResultPayload,
};
use crate::{Tool, ToolContext, ToolOutput};

/// Wraps one existing legacy [`Tool`] so it can be used by the improved runtime.
pub struct LegacyRuntimeToolAdapter {
    inner: Arc<dyn Tool>,
    definition: ToolDefinitionSpec,
}

impl LegacyRuntimeToolAdapter {
    pub fn new(inner: Arc<dyn Tool>, definition: ToolDefinitionSpec) -> Self {
        Self { inner, definition }
    }
}

#[async_trait]
impl RuntimeTool for LegacyRuntimeToolAdapter {
    fn definition(&self) -> ToolDefinitionSpec {
        self.definition.clone()
    }

    async fn validate(&self, _input: &serde_json::Value) -> Result<(), super::ToolInputError> {
        Ok(())
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ToolExecutionContext,
        _reporter: Arc<dyn ToolProgressReporter>,
    ) -> Result<ToolExecutionOutcome, ToolExecuteError> {
        let legacy_ctx = ToolContext {
            cwd: ctx.cwd,
            permissions: ctx.permissions,
            session_id: ctx.session_id,
        };
        let output = self
            .inner
            .execute(&legacy_ctx, input)
            .await
            .map_err(|error| ToolExecuteError::ExecutionFailed {
                message: error.to_string(),
            })?;
        Ok(map_legacy_output(output))
    }
}

pub(super) fn map_execute_error(error: ToolExecuteError) -> ToolExecutionOutcome {
    match error {
        ToolExecuteError::ApprovalRequired { message }
        | ToolExecuteError::PermissionDenied { message } => {
            ToolExecutionOutcome::Denied(ToolDenied { reason: message })
        }
        ToolExecuteError::Interrupted { .. } => ToolExecutionOutcome::Interrupted,
        ToolExecuteError::UnknownTool { tool_name } => ToolExecutionOutcome::Failed(ToolFailure {
            code: "unknown_tool".into(),
            message: format!("unknown tool: {tool_name}"),
        }),
        ToolExecuteError::InvalidInput { message } => ToolExecutionOutcome::Failed(ToolFailure {
            code: "invalid_input".into(),
            message,
        }),
        ToolExecuteError::SandboxUnavailable { message } => {
            ToolExecutionOutcome::Failed(ToolFailure {
                code: "sandbox_unavailable".into(),
                message,
            })
        }
        ToolExecuteError::ExecutionFailed { message } => {
            ToolExecutionOutcome::Failed(ToolFailure {
                code: "execution_failed".into(),
                message,
            })
        }
        ToolExecuteError::Timeout { message } => ToolExecutionOutcome::Failed(ToolFailure {
            code: "timeout".into(),
            message,
        }),
        ToolExecuteError::Internal { message } => ToolExecutionOutcome::Failed(ToolFailure {
            code: "internal".into(),
            message,
        }),
    }
}

pub(super) fn map_legacy_output(output: ToolOutput) -> ToolExecutionOutcome {
    if output.is_error {
        return ToolExecutionOutcome::Failed(ToolFailure {
            code: "tool_error".into(),
            message: output.content,
        });
    }

    ToolExecutionOutcome::Completed(ToolResultPayload {
        content: map_legacy_content(output.content, output.metadata),
        metadata: ToolResultMetadata::default(),
    })
}

fn map_legacy_content(content: String, metadata: Option<serde_json::Value>) -> ToolContent {
    match (content.is_empty(), metadata) {
        (true, Some(json)) => ToolContent::Json(json),
        (false, Some(json)) => ToolContent::Mixed {
            text: Some(content),
            json: Some(json),
        },
        (_, None) => ToolContent::Text(content),
    }
}
