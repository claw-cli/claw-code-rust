use serde::{Deserialize, Serialize};

/// Selects the model used for safety-policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyPolicyModelSelection {
    /// Use the active turn model for compaction summaries.
    UseTurnModel,
    /// Use a separately configured auxiliary model for safety classification.
    UseAxiliaryModel,
}
