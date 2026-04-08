use std::collections::VecDeque;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clawcr_core::{ItemId, SessionId, TurnId, TurnStatus, TurnUsage};
use serde::{Deserialize, Serialize};

/// Stores one turn summary projected onto the server API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSummary {
    /// The stable turn identifier.
    pub turn_id: TurnId,
    /// The owning session identifier.
    pub session_id: SessionId,
    /// The turn sequence number within the session.
    pub sequence: u32,
    /// The current canonical turn status.
    pub status: TurnStatus,
    /// The resolved model slug used by the turn.
    pub model_slug: String,
    /// The time when the turn started.
    pub started_at: DateTime<Utc>,
    /// The time when the turn completed, if it has reached a terminal state.
    pub completed_at: Option<DateTime<Utc>>,
    /// The authoritative provider usage for this turn, when available.
    pub usage: Option<TurnUsage>,
}

/// Describes an input item accepted by the runtime API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputItem {
    /// A plain text input item.
    Text {
        /// The text payload.
        text: String,
    },
    /// A skill reference input item.
    Skill {
        /// The referenced skill identifier.
        id: String,
    },
    /// A local image reference input item.
    LocalImage {
        /// The absolute filesystem path to the image.
        path: PathBuf,
    },
    /// A structured mention input item.
    Mention {
        /// The mention target path or resource URI.
        path: String,
        /// The human-readable mention label.
        name: Option<String>,
    },
}

/// Describes the payload for `turn/start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnStartParams {
    /// The session receiving the new turn.
    pub session_id: SessionId,
    /// The primary user input for the turn.
    pub input: Vec<InputItem>,
    /// An optional requested model slug override.
    pub model: Option<String>,
    /// An optional sandbox override description.
    pub sandbox: Option<String>,
    /// An optional approval-policy override description.
    pub approval_policy: Option<String>,
    /// An optional working-directory override.
    pub cwd: Option<PathBuf>,
}

/// Describes the accepted result returned by `turn/start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnStartResult {
    /// The created turn identifier.
    pub turn_id: TurnId,
    /// The initial accepted turn status.
    pub status: TurnStatus,
    /// The time when the turn was accepted.
    pub accepted_at: DateTime<Utc>,
}

/// Describes the payload for `turn/interrupt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnInterruptParams {
    /// The session that owns the turn.
    pub session_id: SessionId,
    /// The turn being interrupted.
    pub turn_id: TurnId,
    /// An optional human-readable interruption reason.
    pub reason: Option<String>,
}

/// Describes the payload returned by `turn/interrupt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnInterruptResult {
    /// The interrupted turn identifier.
    pub turn_id: TurnId,
    /// The terminal interruption status.
    pub status: TurnStatus,
}

/// Describes the payload for `turn/steer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSteerParams {
    /// The session containing the active turn.
    pub session_id: SessionId,
    /// The turn identifier the client expects to still be active.
    pub expected_turn_id: TurnId,
    /// Additional same-turn user input.
    pub input: Vec<InputItem>,
}

/// Describes the response returned by `turn/steer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSteerResult {
    /// The turn that accepted the steering input.
    pub turn_id: TurnId,
}

/// Identifies the coarse kind of runtime turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnKind {
    /// A normal steerable turn.
    Regular,
    /// A review-focused turn.
    Review,
    /// A manual context-compaction turn.
    ManualCompaction,
    /// Another specialized turn kind.
    Other(String),
}

/// Stores one queued same-turn steering input record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteerInputRecord {
    /// The persisted user-input item identifier for this steering input.
    pub item_id: ItemId,
    /// The time when the steering input was received.
    pub received_at: DateTime<Utc>,
    /// The queued input items.
    pub input: Vec<InputItem>,
}

/// Stores one same-turn steering state bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTurnSteeringState {
    /// The active turn identifier.
    pub turn_id: TurnId,
    /// The kind of turn that is currently executing.
    pub turn_kind: TurnKind,
    /// Steering input queued for the next safe runtime checkpoint.
    pub pending_inputs: VecDeque<SteerInputRecord>,
}
