use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clawcr_core::{SessionId, SessionTitleState};
use serde::{Deserialize, Serialize};

use crate::turn::TurnSummary;

/// Stores the runtime-level status of one session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRuntimeStatus {
    /// The session is loaded with no active turn.
    Idle,
    /// The session currently owns one active turn.
    ActiveTurn,
    /// The session is waiting on client interaction such as approval.
    WaitingClient,
    /// The session is archived.
    Archived,
    /// The session is not loaded in memory.
    Unloaded,
}

/// Stores one session summary projected onto the server API surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    /// The stable session identifier.
    pub session_id: SessionId,
    /// The working directory associated with the session.
    pub cwd: PathBuf,
    /// The timestamp when the session was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of the last known session update.
    pub updated_at: DateTime<Utc>,
    /// The current best-known session title.
    pub title: Option<String>,
    /// The lifecycle state for the current title.
    pub title_state: SessionTitleState,
    /// Whether the session is ephemeral.
    pub ephemeral: bool,
    /// The latest resolved model slug for the session.
    pub resolved_model: Option<String>,
    /// The total provider-reported input tokens accumulated for the session.
    pub total_input_tokens: usize,
    /// The total provider-reported output tokens accumulated for the session.
    pub total_output_tokens: usize,
    /// The current runtime status visible to API clients.
    pub status: SessionRuntimeStatus,
}

/// Describes the payload for `session/start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStartParams {
    /// The working directory for the new session.
    pub cwd: PathBuf,
    /// Whether the session should be treated as ephemeral.
    pub ephemeral: bool,
    /// The explicit title to assign at creation time, if any.
    pub title: Option<String>,
    /// An optional requested model slug.
    pub model: Option<String>,
}

/// Describes the response returned by `session/start`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStartResult {
    /// The created session identifier.
    pub session_id: SessionId,
    /// The session creation timestamp.
    pub created_at: DateTime<Utc>,
    /// The working directory assigned to the session.
    pub cwd: PathBuf,
    /// Whether the session is ephemeral.
    pub ephemeral: bool,
    /// The model resolved for the initial session state.
    pub resolved_model: Option<String>,
}

/// Describes the payload for `session/resume`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionResumeParams {
    /// The session identifier that should be resumed.
    pub session_id: SessionId,
}

/// Describes the response returned by `session/resume`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionResumeResult {
    /// The resumed session summary visible to the client.
    pub session: SessionSummary,
    /// The latest turn for the session, when one exists.
    pub latest_turn: Option<TurnSummary>,
    /// The number of items loaded while resuming the session.
    pub loaded_item_count: u64,
    /// Replay-friendly history items used by interactive clients to rebuild the transcript.
    pub history_items: Vec<SessionHistoryItem>,
}

/// Visual category for one replayed session-history item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionHistoryItemKind {
    /// User-authored prompt text.
    User,
    /// Assistant-authored text.
    Assistant,
    /// Tool invocation summary.
    ToolCall,
    /// Successful tool output.
    ToolResult,
    /// Failed tool output.
    Error,
}

/// One replay-friendly transcript item returned when resuming a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHistoryItem {
    /// Stable visual category used by the client to render the item.
    pub kind: SessionHistoryItemKind,
    /// Short display title shown for tool and error items.
    pub title: String,
    /// Main text body rendered in the transcript.
    pub body: String,
}

/// Describes the payload for `session/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionListParams {}

/// Describes the response returned by `session/list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionListResult {
    /// Sessions known to the server, sorted by most recent update first.
    pub sessions: Vec<SessionSummary>,
}

/// Describes the payload for `session/title/update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTitleUpdateParams {
    /// The session whose title should be changed.
    pub session_id: SessionId,
    /// The new explicit user-facing title.
    pub title: String,
}

/// Describes the response returned by `session/title/update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTitleUpdateResult {
    /// The updated session summary visible to the client.
    pub session: SessionSummary,
}

/// Describes the payload for `session/fork`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionForkParams {
    /// The source session that should be forked.
    pub session_id: SessionId,
    /// The explicit title for the forked session, when provided.
    pub title: Option<String>,
    /// The optional working-directory override for the fork.
    pub cwd: Option<PathBuf>,
}

/// Describes the response returned by `session/fork`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionForkResult {
    /// The forked session summary visible to the client.
    pub session: SessionSummary,
    /// The source session identifier.
    pub forked_from_session_id: SessionId,
}
