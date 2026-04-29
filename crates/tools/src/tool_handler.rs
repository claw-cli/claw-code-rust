use async_trait::async_trait;

use crate::errors::ToolExecutionError;
use crate::handler_kind::ToolHandlerKind;
use crate::invocation::{ToolInvocation, ToolOutput};

#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn tool_kind(&self) -> ToolHandlerKind;

    async fn handle(
        &self,
        invocation: ToolInvocation,
    ) -> Result<Box<dyn ToolOutput>, ToolExecutionError>;
}
