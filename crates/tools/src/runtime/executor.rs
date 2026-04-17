use std::sync::Arc;

use futures::stream::{self, StreamExt};

use crate::runtime::{
    NullToolProgressReporter, RuntimeToolRegistry, ToolDenied, ToolExecutionContext,
    ToolExecutionMode, ToolExecutionOutcome, ToolExecutionRecord, ToolFailure, ToolInvocation,
    ToolProgressReporter,
};

/// Minimal executor for the runtime layer.
///
/// Read-only tools run in bounded parallel. Mutating tools remain sequential.
/// Streaming assembly remains in adapters, and broader orchestration changes
/// will happen later.
pub struct RuntimeToolExecutor {
    registry: Arc<RuntimeToolRegistry>,
}

impl RuntimeToolExecutor {
    pub fn new(registry: Arc<RuntimeToolRegistry>) -> Self {
        Self { registry }
    }

    pub async fn execute_batch(
        &self,
        invocations: &[ToolInvocation],
        ctx: &ToolExecutionContext,
    ) -> Vec<ToolExecutionRecord> {
        let reporter: Arc<dyn ToolProgressReporter> = Arc::new(NullToolProgressReporter);
        let mut results = vec![None; invocations.len()];
        let mut pending_read_only = Vec::new();

        for (index, invocation) in invocations.iter().enumerate() {
            match self.execution_mode_for(&invocation.tool_name) {
                Some(ToolExecutionMode::ReadOnly) => {
                    pending_read_only.push((index, invocation.clone()));
                }
                _ => {
                    self.flush_read_only_batch(
                        &mut results,
                        &mut pending_read_only,
                        ctx,
                        &reporter,
                    )
                    .await;
                    results[index] = Some(
                        self.execute_invocation(invocation, ctx, Arc::clone(&reporter))
                            .await,
                    );
                }
            }
        }

        self.flush_read_only_batch(&mut results, &mut pending_read_only, ctx, &reporter)
            .await;

        results
            .into_iter()
            .map(|result| result.expect("every tool invocation should produce one result"))
            .collect()
    }

    pub async fn execute_invocation(
        &self,
        invocation: &ToolInvocation,
        ctx: &ToolExecutionContext,
        reporter: Arc<dyn ToolProgressReporter>,
    ) -> ToolExecutionRecord {
        let Some(tool) = self.registry.get(&invocation.tool_name) else {
            return ToolExecutionRecord {
                tool_call_id: invocation.tool_call_id.clone(),
                tool_name: invocation.tool_name.clone(),
                outcome: ToolExecutionOutcome::Failed(ToolFailure {
                    code: "unknown_tool".into(),
                    message: format!("unknown tool: {}", invocation.tool_name.0),
                }),
            };
        };

        if !self
            .registry
            .is_enabled(ctx.app_config.as_ref(), &invocation.tool_name)
        {
            return ToolExecutionRecord {
                tool_call_id: invocation.tool_call_id.clone(),
                tool_name: invocation.tool_name.clone(),
                outcome: ToolExecutionOutcome::Denied(ToolDenied {
                    reason: format!("tool is disabled: {}", invocation.tool_name.0),
                }),
            };
        }

        if let Err(error) = tool.validate(&invocation.input).await {
            return ToolExecutionRecord {
                tool_call_id: invocation.tool_call_id.clone(),
                tool_name: invocation.tool_name.clone(),
                outcome: ToolExecutionOutcome::Failed(ToolFailure {
                    code: "invalid_input".into(),
                    message: error.to_string(),
                }),
            };
        }

        let outcome = match tool
            .execute(invocation.input.clone(), ctx.clone(), reporter)
            .await
        {
            Ok(outcome) => outcome,
            Err(error) => super::legacy::map_execute_error(error),
        };

        ToolExecutionRecord {
            tool_call_id: invocation.tool_call_id.clone(),
            tool_name: invocation.tool_name.clone(),
            outcome,
        }
    }

    fn execution_mode_for(&self, name: &crate::runtime::ToolName) -> Option<ToolExecutionMode> {
        self.registry
            .get(name)
            .map(|tool| tool.definition().execution_mode)
    }

    async fn flush_read_only_batch(
        &self,
        results: &mut [Option<ToolExecutionRecord>],
        pending: &mut Vec<(usize, ToolInvocation)>,
        ctx: &ToolExecutionContext,
        reporter: &Arc<dyn ToolProgressReporter>,
    ) {
        if pending.is_empty() {
            return;
        }

        let concurrency = usize::from(ctx.app_config.max_parallel_read_tools.max(1));
        let batch = std::mem::take(pending);
        let records = stream::iter(batch.into_iter().map(|(index, invocation)| {
            let reporter = Arc::clone(reporter);
            async move {
                let record = self.execute_invocation(&invocation, ctx, reporter).await;
                (index, record)
            }
        }))
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>()
        .await;

        for (index, record) in records {
            results[index] = Some(record);
        }
    }
}
