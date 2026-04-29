use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(pub SmolStr);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolCallId(pub String);

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub call_id: ToolCallId,
    pub tool_name: ToolName,
    pub session_id: String,
    pub cwd: PathBuf,
    pub input: serde_json::Value,
}

pub trait ToolOutput: Send {
    fn to_content(self: Box<Self>) -> ToolContent;
    fn is_error(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolContent {
    Text(String),
    Json(serde_json::Value),
    Mixed {
        text: Option<String>,
        json: Option<serde_json::Value>,
    },
}

impl ToolContent {
    pub fn into_string(self) -> String {
        match self {
            ToolContent::Text(t) => t,
            ToolContent::Json(v) => v.to_string(),
            ToolContent::Mixed { text, json } => {
                let mut parts = Vec::new();
                if let Some(t) = text {
                    parts.push(t);
                }
                if let Some(j) = json {
                    parts.push(j.to_string());
                }
                parts.join("\n")
            }
        }
    }
}

pub struct FunctionToolOutput {
    pub content: ToolContent,
    pub is_error: bool,
}

impl FunctionToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        FunctionToolOutput {
            content: ToolContent::Text(content.into()),
            is_error: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        FunctionToolOutput {
            content: ToolContent::Text(message.into()),
            is_error: true,
        }
    }

    pub fn from_output(output: crate::ToolOutput) -> Self {
        FunctionToolOutput {
            content: if output.is_error {
                ToolContent::Text(output.content)
            } else {
                match output.metadata {
                    Some(meta) => ToolContent::Mixed {
                        text: Some(output.content),
                        json: Some(meta),
                    },
                    None => ToolContent::Text(output.content),
                }
            },
            is_error: output.is_error,
        }
    }
}

impl ToolOutput for FunctionToolOutput {
    fn to_content(self: Box<Self>) -> ToolContent {
        self.content
    }

    fn is_error(&self) -> bool {
        self.is_error
    }
}
