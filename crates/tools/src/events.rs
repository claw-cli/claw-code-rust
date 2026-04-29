use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolEvent {
    ToolCallBegin {
        tool_name: String,
        call_id: String,
    },
    ToolCallEnd {
        tool_name: String,
        call_id: String,
        duration_ms: u64,
    },
    ToolProgress {
        tool_name: String,
        call_id: String,
        message: String,
    },
}
