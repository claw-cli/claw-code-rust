use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ToolExecutionError {
    #[error("permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("timeout: {message}")]
    Timeout { message: String },

    #[error("interrupted")]
    Interrupted,

    #[error("internal: {message}")]
    Internal { message: String },
}

#[derive(Debug, Clone, Error)]
pub enum ToolDispatchError {
    #[error("unknown tool: {name}")]
    UnknownTool { name: String },

    #[error("{0}")]
    ExecutionError(#[from] ToolExecutionError),
}
