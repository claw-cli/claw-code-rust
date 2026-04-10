use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, Command},
    sync::{Mutex, mpsc, oneshot},
    time::{Duration, timeout},
};

use crate::{
    ClientNotification, ClientRequest, ErrorResponse, InitializeParams, InitializeResult,
    NotificationEnvelope, ProtocolErrorCode, ServerEvent, SessionForkParams, SessionForkResult,
    SessionListParams, SessionListResult, SessionResumeParams, SessionResumeResult,
    SessionStartParams, SessionStartResult, SessionTitleUpdateParams, SessionTitleUpdateResult,
    SuccessResponse, TurnInterruptParams, TurnInterruptResult, TurnStartParams, TurnStartResult,
    TurnSteerParams, TurnSteerResult,
};

/// Immutable launch configuration for one stdio-connected server client.
#[derive(Debug, Clone)]
pub struct StdioServerClientConfig {
    /// Absolute path to the executable that should be launched.
    pub program: PathBuf,
    /// Optional workspace root forwarded to the server process.
    pub workspace_root: Option<PathBuf>,
    /// Environment overrides applied only to the spawned server process.
    pub env: Vec<(String, String)>,
}

/// One server notification delivered over the stdio transport.
#[derive(Debug, Clone)]
pub struct ServerNotificationMessage {
    /// The exact notification method name.
    pub method: String,
    /// The untyped JSON payload for the notification.
    pub params: serde_json::Value,
}

/// Thin stdio JSON client for the transport-facing server runtime.
pub struct StdioServerClient {
    /// Child process that owns the server runtime.
    child: Child,
    /// Writable stdin pipe for client-to-server requests.
    stdin: ChildStdin,
    /// Shared map of request waiters keyed by request identifier.
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
    /// Monotonic request identifier counter.
    next_request_id: AtomicU64,
    /// Receiver for outbound server notifications.
    notifications_rx: mpsc::UnboundedReceiver<ServerNotificationMessage>,
}

impl StdioServerClient {
    /// Spawns a new stdio-connected `clawcr server` process.
    pub async fn spawn(config: StdioServerClientConfig) -> Result<Self> {
        tracing::info!(
            program = %config.program.display(),
            workspace_root = ?config.workspace_root,
            env_override_count = config.env.len(),
            "spawning stdio server client"
        );
        let mut command = Command::new(&config.program);
        command.arg("server");
        if let Some(workspace_root) = config.workspace_root {
            command.arg("--working-root").arg(workspace_root);
        }
        for (key, value) in config.env {
            command.env(key, value);
        }
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn {}", config.program.display()))?;
        let stdin = child.stdin.take().context("capture server stdin")?;
        let stdout = child.stdout.take().context("capture server stdout")?;
        let stderr = child.stderr.take().context("capture server stderr")?;
        let pending = Arc::new(Mutex::new(
            HashMap::<u64, oneshot::Sender<serde_json::Value>>::new(),
        ));
        let (notifications_tx, notifications_rx) = mpsc::unbounded_channel();

        tokio::spawn(run_stdout_reader(
            BufReader::new(stdout).lines(),
            Arc::clone(&pending),
            notifications_tx,
        ));
        tokio::spawn(run_stderr_reader(BufReader::new(stderr).lines()));

        Ok(Self {
            child,
            stdin,
            pending,
            next_request_id: AtomicU64::new(1),
            notifications_rx,
        })
    }

    /// Completes the initialize handshake for a stdio client transport.
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        tracing::info!("initializing stdio server client");
        let result = timeout(
            Duration::from_secs(10),
            self.request(
                "initialize",
                InitializeParams {
                    client_name: "clawcr".into(),
                    client_version: env!("CARGO_PKG_VERSION").into(),
                    transport: crate::ClientTransportKind::Stdio,
                    supports_streaming: true,
                    supports_binary_images: false,
                    opt_out_notification_methods: Vec::new(),
                },
            ),
        )
        .await
        .context("timed out waiting for initialize response from server")??;
        self.notify("initialized", serde_json::json!({})).await?;
        tracing::info!("stdio server client initialized");
        Ok(result)
    }

    /// Starts a new server session and returns the typed result payload.
    pub async fn session_start(
        &mut self,
        params: SessionStartParams,
    ) -> Result<SessionStartResult> {
        self.request("session/start", params).await
    }

    /// Resumes an existing session and returns the typed result payload.
    pub async fn session_resume(
        &mut self,
        params: SessionResumeParams,
    ) -> Result<SessionResumeResult> {
        self.request("session/resume", params).await
    }

    /// Lists sessions currently known to the server.
    pub async fn session_list(&mut self, params: SessionListParams) -> Result<SessionListResult> {
        self.request("session/list", params).await
    }

    /// Updates the title for one persisted or in-memory session.
    pub async fn session_title_update(
        &mut self,
        params: SessionTitleUpdateParams,
    ) -> Result<SessionTitleUpdateResult> {
        self.request("session/title/update", params).await
    }

    /// Forks an existing session and returns the typed result payload.
    pub async fn session_fork(&mut self, params: SessionForkParams) -> Result<SessionForkResult> {
        self.request("session/fork", params).await
    }

    /// Starts one turn for an existing session.
    pub async fn turn_start(&mut self, params: TurnStartParams) -> Result<TurnStartResult> {
        self.request("turn/start", params).await
    }

    /// Interrupts one active turn.
    pub async fn turn_interrupt(
        &mut self,
        params: TurnInterruptParams,
    ) -> Result<TurnInterruptResult> {
        self.request("turn/interrupt", params).await
    }

    /// Sends one same-turn steering request.
    pub async fn turn_steer(&mut self, params: TurnSteerParams) -> Result<TurnSteerResult> {
        self.request("turn/steer", params).await
    }

    /// Receives the next server notification emitted on the connection.
    pub async fn recv_notification(&mut self) -> Option<ServerNotificationMessage> {
        self.notifications_rx.recv().await
    }

    /// Attempts to deserialize the next server event notification.
    pub async fn recv_event(&mut self) -> Result<Option<(String, ServerEvent)>> {
        let Some(notification) = self.recv_notification().await else {
            return Ok(None);
        };
        let event = serde_json::from_value(notification.params.clone()).with_context(|| {
            format!(
                "failed to decode server event for method {}",
                notification.method
            )
        })?;
        Ok(Some((notification.method, event)))
    }

    /// Stops the child server process and waits for it to exit.
    pub async fn shutdown(mut self) -> Result<()> {
        let _ = self.stdin.shutdown().await;
        self.child.kill().await.ok();
        let _ = self.child.wait().await;
        Ok(())
    }

    async fn request<P, R>(&mut self, method: &str, params: P) -> Result<R>
    where
        P: serde::Serialize,
        R: DeserializeOwned,
    {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        tracing::debug!(request_id, method, "sending client request");
        let (response_tx, response_rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id, response_tx);
        self.write_json(&ClientRequest {
            id: serde_json::json!(request_id),
            method: method.to_string(),
            params,
        })
        .await?;

        let response = timeout(Duration::from_secs(10), response_rx)
            .await
            .with_context(|| {
                format!("timed out waiting for server response to request {request_id}")
            })?
            .with_context(|| format!("server dropped response for request {request_id}"))?;
        tracing::debug!(request_id, method, "received client response");
        if response.get("error").is_some() {
            let error: ErrorResponse =
                serde_json::from_value(response).context("decode error response from server")?;
            let data = if error.error.data.is_null() {
                String::new()
            } else {
                format!(" data={}", error.error.data)
            };
            anyhow::bail!(
                "server {}: {}{}",
                format_protocol_error_code(&error.error.code),
                error.error.message,
                data
            );
        }
        let success: SuccessResponse<R> =
            serde_json::from_value(response).context("decode success response from server")?;
        Ok(success.result)
    }

    async fn notify<P>(&mut self, method: &str, params: P) -> Result<()>
    where
        P: serde::Serialize,
    {
        self.write_json(&ClientNotification {
            method: method.to_string(),
            params,
        })
        .await
    }

    async fn write_json<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let line = serde_json::to_vec(value).context("serialize client request")?;
        self.stdin
            .write_all(&line)
            .await
            .context("write request to server stdin")?;
        self.stdin
            .write_all(b"\n")
            .await
            .context("write request newline to server stdin")?;
        self.stdin.flush().await.context("flush server stdin")?;
        Ok(())
    }
}

async fn run_stdout_reader(
    mut lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>>,
    notifications_tx: mpsc::UnboundedSender<ServerNotificationMessage>,
) {
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            tracing::warn!(line = %line, "failed to parse JSON from server stdout");
            continue;
        };
        if let Some(request_id) = value.get("id").and_then(serde_json::Value::as_u64) {
            if let Some(sender) = pending.lock().await.remove(&request_id) {
                let _ = sender.send(value);
            }
            continue;
        }
        let Ok(notification) =
            serde_json::from_value::<NotificationEnvelope<serde_json::Value>>(value)
        else {
            continue;
        };
        let _ = notifications_tx.send(ServerNotificationMessage {
            method: notification.method,
            params: notification.params,
        });
    }
    tracing::warn!("server stdout reader stopped");
}

async fn run_stderr_reader(mut lines: tokio::io::Lines<BufReader<ChildStderr>>) {
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        tracing::warn!(server_stderr = %trimmed, "server child stderr");
    }
    tracing::warn!("server stderr reader stopped");
}

fn format_protocol_error_code(code: &ProtocolErrorCode) -> &'static str {
    match code {
        ProtocolErrorCode::NotInitialized => "NotInitialized",
        ProtocolErrorCode::InvalidParams => "InvalidParams",
        ProtocolErrorCode::SessionNotFound => "SessionNotFound",
        ProtocolErrorCode::TurnNotFound => "TurnNotFound",
        ProtocolErrorCode::TurnAlreadyRunning => "TurnAlreadyRunning",
        ProtocolErrorCode::ApprovalNotFound => "ApprovalNotFound",
        ProtocolErrorCode::PolicyDenied => "PolicyDenied",
        ProtocolErrorCode::ContextLimitExceeded => "ContextLimitExceeded",
        ProtocolErrorCode::NoActiveTurn => "NoActiveTurn",
        ProtocolErrorCode::ExpectedTurnMismatch => "ExpectedTurnMismatch",
        ProtocolErrorCode::ActiveTurnNotSteerable => "ActiveTurnNotSteerable",
        ProtocolErrorCode::EmptyInput => "EmptyInput",
        ProtocolErrorCode::InternalError => "InternalError",
    }
}
