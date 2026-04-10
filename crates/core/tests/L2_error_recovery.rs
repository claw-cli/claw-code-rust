//! L2: Error recovery scenario tests.
//!
//! These tests define the expected behavior for P0 error recovery capabilities
//! that are NOT yet implemented. They are marked `#[ignore]` and serve as
//! behavioral contracts — remove `#[ignore]` as each feature lands.
//!
//! Capability mapping:
//!   - 1.5  ContextTooLong recovery
//!   - 1.6  MaxOutputTokens auto-continue
//!   - 3.2  Stream error classification (429, 5xx, auth)
//!   - (bonus) Malformed tool JSON graceful fallback

#[allow(dead_code, unused_imports)]
mod harness;

use std::sync::Arc;

use serde_json::json;

use clawcr_provider::StopReason;
use clawcr_tools::{ToolOrchestrator, ToolRegistry};

use clawcr_core::{AgentError, ContentBlock, Message, query};

use harness::builders::*;
use harness::{ScriptedProvider, SpyTool, event_collector};

fn setup_registry() -> (Arc<ToolRegistry>, ToolOrchestrator) {
    let registry = Arc::new(ToolRegistry::new());
    let orchestrator = ToolOrchestrator::new(Arc::clone(&registry));
    (registry, orchestrator)
}

fn setup_registry_with_tool(tool: SpyTool) -> (Arc<ToolRegistry>, ToolOrchestrator) {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(tool));
    let registry = Arc::new(reg);
    let orchestrator = ToolOrchestrator::new(Arc::clone(&registry));
    (registry, orchestrator)
}

// ---------------------------------------------------------------------------
// 1.5 ContextTooLong recovery
// ---------------------------------------------------------------------------

/// When the provider returns a context_too_long error, the query loop should
/// automatically compact the conversation history and retry, rather than
/// propagating the error immediately.
#[tokio::test]
async fn context_too_long_triggers_compact() {
    let provider = ScriptedProvider::builder()
        .turn_error(anyhow::anyhow!("context_too_long"))
        .turn(make_text_turn(
            "recovered after compact",
            StopReason::EndTurn,
        ))
        .build();
    let captured = provider.captured_requests.clone();
    let (registry, orchestrator) = setup_registry();

    let mut session = make_session();
    // Pad the session with enough messages to be compactable
    for i in 0..20 {
        session.push_message(Message::user(format!("message {}", i)));
        session.push_message(Message::assistant_text(format!("reply {}", i)));
    }

    let result = query(&mut session, &provider, registry, &orchestrator, None).await;
    assert!(
        result.is_ok(),
        "should recover via compact, got: {:?}",
        result
    );

    // The second request should have fewer messages than the first attempt
    let requests = captured.lock().unwrap();
    assert!(requests.len() >= 2);
    assert!(
        requests[1].messages.len() < requests[0].messages.len(),
        "compacted request should have fewer messages"
    );
}

/// If compaction still can't bring the context under the limit, the query
/// should return ContextTooLong rather than looping forever.
#[tokio::test]
async fn context_too_long_after_compact_fails() {
    let provider = ScriptedProvider::builder()
        .turn_error(anyhow::anyhow!("context_too_long"))
        .turn_error(anyhow::anyhow!("context_too_long"))
        .build();
    let (registry, orchestrator) = setup_registry();

    let mut session = make_session();

    let result = query(&mut session, &provider, registry, &orchestrator, None).await;
    match result {
        Err(AgentError::ContextTooLong) => {} // expected
        other => panic!("expected ContextTooLong, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 1.6 MaxOutputTokens auto-continue
// ---------------------------------------------------------------------------

/// When the model stops with MaxTokens and there are no tool calls, the query
/// loop should automatically inject a "please continue" user message and
/// request a follow-up from the model.
#[tokio::test]
async fn max_output_tokens_auto_continue() {
    let provider = ScriptedProvider::builder()
        // Turn 1: model is cut off mid-text
        .turn(make_text_turn("partial response...", StopReason::MaxTokens))
        // Turn 2: model continues after auto-injected prompt
        .turn(make_text_turn(
            " and here is the rest.",
            StopReason::EndTurn,
        ))
        .build();
    let captured = provider.captured_requests.clone();
    let (registry, orchestrator) = setup_registry();
    let (callback, _events) = event_collector();

    let mut session = make_session();
    let result = query(
        &mut session,
        &provider,
        registry,
        &orchestrator,
        Some(callback),
    )
    .await;
    assert!(result.is_ok());

    // Two requests should have been made
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 2);

    // The second request should include a continuation prompt from the user
    let last_msg = requests[1].messages.last().unwrap();
    assert_eq!(last_msg.role, "user");

    // Session should have both text fragments
    assert_eq!(session.turn_count, 2);
}

/// When the model stops with MaxTokens but has tool calls, tool execution
/// takes priority over auto-continue.
#[tokio::test]
async fn max_output_tokens_with_tool_use() {
    let spy = SpyTool::new("my_tool", false);

    let provider = ScriptedProvider::builder()
        // MaxTokens but with a tool call — should execute tool, not auto-continue
        .turn(make_tool_turn(
            "t1",
            "my_tool",
            json!({"action": "do_thing"}),
            StopReason::MaxTokens,
        ))
        .turn(make_text_turn("done", StopReason::EndTurn))
        .build();

    let (registry, orchestrator) = setup_registry_with_tool(spy);
    let mut session = make_session();

    let result = query(&mut session, &provider, registry, &orchestrator, None).await;
    assert!(result.is_ok());

    // Tool result should be present (tool was executed, not auto-continue)
    let has_tool_result = session.messages.iter().any(|m| {
        m.content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolResult { .. }))
    });
    assert!(
        has_tool_result,
        "tool should be executed when MaxTokens + tool_use"
    );
}

// ---------------------------------------------------------------------------
// 3.2 Stream error classification
// ---------------------------------------------------------------------------

/// 429 rate-limit errors should be retried with backoff rather than
/// propagated immediately.
#[tokio::test]
async fn stream_error_429_retries() {
    let provider = ScriptedProvider::builder()
        .turn_error(anyhow::anyhow!("429 Too Many Requests"))
        .turn_error(anyhow::anyhow!("429 Too Many Requests"))
        .turn(make_text_turn("success after retry", StopReason::EndTurn))
        .build();
    let captured = provider.captured_requests.clone();
    let (registry, orchestrator) = setup_registry();

    let mut session = make_session();
    let result = query(&mut session, &provider, registry, &orchestrator, None).await;

    assert!(result.is_ok(), "should succeed after retrying 429");
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 3, "should have made 3 attempts");
}

/// 5xx server errors should also be retried.
#[tokio::test]
async fn stream_error_5xx_retries() {
    let provider = ScriptedProvider::builder()
        .turn_error(anyhow::anyhow!("500 Internal Server Error"))
        .turn(make_text_turn("recovered", StopReason::EndTurn))
        .build();
    let (registry, orchestrator) = setup_registry();

    let mut session = make_session();
    let result = query(&mut session, &provider, registry, &orchestrator, None).await;

    assert!(result.is_ok(), "should succeed after retrying 5xx");
}

/// 401 authentication errors should fail immediately without retrying.
#[tokio::test]
async fn stream_error_auth_fails_fast() {
    let provider = ScriptedProvider::builder()
        .turn_error(anyhow::anyhow!("401 Unauthorized"))
        // If this second turn is consumed, the test should fail
        .turn(make_text_turn("should not reach", StopReason::EndTurn))
        .build();
    let captured = provider.captured_requests.clone();
    let (registry, orchestrator) = setup_registry();

    let mut session = make_session();
    let result = query(&mut session, &provider, registry, &orchestrator, None).await;

    assert!(result.is_err(), "401 should not be retried");
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 1, "should have made only 1 attempt");
}

// ---------------------------------------------------------------------------
// Bonus: malformed tool JSON graceful fallback
// ---------------------------------------------------------------------------

/// When InputJsonDelta chunks produce invalid JSON, the tool call input
/// should fall back to `{}` instead of panicking. This tests the existing
/// `unwrap_or` behavior in query.rs.
#[tokio::test]
async fn malformed_tool_json_graceful() {
    let spy = SpyTool::new("my_tool", false);

    let provider = ScriptedProvider::builder()
        .turn(make_tool_turn_with_json_chunks(
            "t1",
            "my_tool",
            &["{not valid", " json at all}}}"],
            json!({}), // full_input is irrelevant; the chunks are what matter
            StopReason::ToolUse,
        ))
        .turn(make_text_turn("done", StopReason::EndTurn))
        .build();

    let (registry, orchestrator) = setup_registry_with_tool(spy);
    let mut session = make_session();

    // Should not panic — invalid JSON falls back to {}
    let result = query(&mut session, &provider, registry, &orchestrator, None).await;
    assert!(result.is_ok());

    // The assistant message ToolUse should have fallen back to empty object
    let assistant_msg = &session.messages[1];
    let tool_input = assistant_msg
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::ToolUse { input, .. } => Some(input),
            _ => None,
        })
        .expect("should have ToolUse block");

    assert_eq!(
        tool_input,
        &json!({}),
        "invalid JSON should fall back to empty object"
    );
}
