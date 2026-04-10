pub mod builders;

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde_json::json;

use clawcr_provider::{ModelProvider, ModelRequest, ModelResponse, StreamEvent};
use clawcr_tools::{Tool, ToolContext, ToolOutput};

use clawcr_core::QueryEvent;

// ---------------------------------------------------------------------------
// ScriptedProvider — plays back a pre-recorded sequence of stream turns
// ---------------------------------------------------------------------------

/// Each turn is either a successful stream of events or an error returned from
/// `stream()` itself (simulating connection / auth failures).
pub enum TurnScript {
    Events(Vec<Result<StreamEvent>>),
    StreamError(anyhow::Error),
}

pub struct ScriptedProvider {
    scripts: Mutex<VecDeque<TurnScript>>,
    pub captured_requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl ScriptedProvider {
    pub fn builder() -> ScriptedProviderBuilder {
        ScriptedProviderBuilder {
            turns: VecDeque::new(),
        }
    }

    pub fn requests(&self) -> Vec<ModelRequest> {
        self.captured_requests.lock().unwrap().clone()
    }
}

pub struct ScriptedProviderBuilder {
    turns: VecDeque<TurnScript>,
}

impl ScriptedProviderBuilder {
    pub fn turn(mut self, events: Vec<Result<StreamEvent>>) -> Self {
        self.turns.push_back(TurnScript::Events(events));
        self
    }

    pub fn turn_error(mut self, err: anyhow::Error) -> Self {
        self.turns.push_back(TurnScript::StreamError(err));
        self
    }

    pub fn build(self) -> ScriptedProvider {
        ScriptedProvider {
            scripts: Mutex::new(self.turns),
            captured_requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl ModelProvider for ScriptedProvider {
    async fn completion(&self, _request: ModelRequest) -> Result<ModelResponse> {
        unreachable!("ScriptedProvider only supports completion_stream()")
    }

    async fn completion_stream(
        &self,
        request: ModelRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>> {
        self.captured_requests.lock().unwrap().push(request);

        let script = self
            .scripts
            .lock()
            .unwrap()
            .pop_front()
            .expect("ScriptedProvider: no more scripted turns");

        match script {
            TurnScript::Events(events) => Ok(Box::pin(futures::stream::iter(events))),
            TurnScript::StreamError(e) => Err(e),
        }
    }

    fn name(&self) -> &str {
        "scripted-test-provider"
    }
}

// ---------------------------------------------------------------------------
// SpyTool — records calls and returns configurable responses
// ---------------------------------------------------------------------------

type ResponseFn = Box<dyn Fn(serde_json::Value) -> ToolOutput + Send + Sync>;

pub struct SpyTool {
    tool_name: String,
    read_only: bool,
    calls: Mutex<Vec<serde_json::Value>>,
    response_fn: ResponseFn,
}

impl SpyTool {
    pub fn new(name: impl Into<String>, read_only: bool) -> Self {
        Self {
            tool_name: name.into(),
            read_only,
            calls: Mutex::new(Vec::new()),
            response_fn: Box::new(|_| ToolOutput::success("ok")),
        }
    }

    pub fn with_response<F>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> ToolOutput + Send + Sync + 'static,
    {
        self.response_fn = Box::new(f);
        self
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn calls(&self) -> Vec<serde_json::Value> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Tool for SpyTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        "A spy tool for testing."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({"type": "object"})
    }

    async fn execute(&self, _ctx: &ToolContext, input: serde_json::Value) -> Result<ToolOutput> {
        self.calls.lock().unwrap().push(input.clone());
        Ok((self.response_fn)(input))
    }

    fn is_read_only(&self) -> bool {
        self.read_only
    }
}

// ---------------------------------------------------------------------------
// EventCollector — captures QueryEvents emitted during a query() run
// ---------------------------------------------------------------------------

pub fn event_collector() -> (clawcr_core::EventCallback, Arc<Mutex<Vec<QueryEvent>>>) {
    let events: Arc<Mutex<Vec<QueryEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);
    let callback: clawcr_core::EventCallback = Arc::new(move |event| {
        events_clone.lock().unwrap().push(event);
    });
    (callback, events)
}
