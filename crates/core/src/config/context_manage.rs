use serde::{Deserialize, Serialize};

/// Stores defaults for context preservation and compaction behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextManageConfig {
    /// The number of most recent turns that must remain un-compacted.
    pub preserve_recent_turns: u32,
    /// The percentage threshold that triggers automatic compaction.
    pub auto_compact_percent: Option<u8>,
    /// Whether the runtime should allow manual compaction requests.
    pub manual_compaction_enabled: bool,
}
