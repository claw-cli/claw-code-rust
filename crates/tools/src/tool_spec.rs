use serde::{Deserialize, Serialize};

use crate::JsonSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolOutputMode {
    StructuredJson,
    Text,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolExecutionMode {
    ReadOnly,
    Mutating,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCapabilityTag {
    ReadFiles,
    WriteFiles,
    ExecuteProcess,
    NetworkAccess,
    SearchWorkspace,
    ReadImages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: JsonSchema,
    pub output_mode: ToolOutputMode,
    pub execution_mode: ToolExecutionMode,
    pub capability_tags: Vec<ToolCapabilityTag>,
    pub supports_parallel: bool,
}
