# Core Runtime (Query Runtime) — Capability Alignment Analysis

> Based on the 12 capabilities in Chapter 1 of `goal.md`, compared one by one with the actual code in `crates/core/src/`, and prioritized by importance.

---

## 1. Priority Ranking

### P0 — Most Critical (affects basic usability)

| Priority | #    | Capability                   | Why it is top priority                                                                                                            |
| -------- | ---- | ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1        | 1.10 | **Interrupt & Resume**       | Ctrl+C kills the process directly; users cannot safely interrupt generation, and accidental interruption loses the entire session |
| 2        | 1.5  | **ContextTooLong Recovery**  | Long conversations will inevitably exceed context limits; without recovery = crash and total context loss                         |
| 3        | 1.3  | **Auto Compact**             | Preventive defense for 1.5 — compress history before overflow; skeleton already exists, relatively low integration cost           |
| 4        | 1.6  | **MaxOutputTokens Recovery** | Truncated output means incomplete code/thoughts; simple fix (append “please continue”)                                            |

---

### P1 — Important (affects productivity & cost)

| Priority | #    | Capability               | Why it matters                                                                                                               |
| -------- | ---- | ------------------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| 5        | 1.7  | **Token Budget Control** | Foundation for cost control; determines when/how to compact. Without it, autoCompact cannot trigger correctly                |
| 6        | 1.4  | **Micro Compact**        | A single tool_result (e.g., large grep output) can consume huge context; local compression improves efficiency significantly |
| 7        | 1.9  | **Memory Prefetch**      | Loading CLAUDE.md provides project-level context, improving model understanding and first response quality                   |
| 8        | 1.11 | **Usage Tracking**       | Users need visibility into usage/cost; also required for optimizing prompt caching                                           |

---

### P2 — Valuable but can be deferred

| Priority | #   | Capability     | Notes                                                                            |
| -------- | --- | -------------- | -------------------------------------------------------------------------------- |
| 9        | 1.8 | **Stop Hooks** | Extensibility feature (auto-commit, auto-test, etc.), not required for core flow |

---

### Already Implemented (no scheduling needed)

| #    | Capability                       |
| ---- | -------------------------------- |
| 1.1  | Multi-turn loop                  |
| 1.2  | Streaming output                 |
| 1.12 | Unified multi-provider streaming |

---

## 2. Code Alignment Check

Compare `goal.md` with actual implementation for accuracy.

---

### 1.1 Multi-turn loop ✅ Matches

* **goal.md**: Implemented
* **Code**: `query.rs` contains full loop: build request → stream → collect tool_use → execute → append tool_result → repeat

---

### 1.2 Streaming output ✅ Matches

* **goal.md**: Implemented
* **Code**: Handles `StreamEvent::TextDelta`, emits `QueryEvent::TextDelta`, printed incrementally in `main.rs`

---

### 1.3 Auto Compact ✅ Matches

* **goal.md**: Exists but not integrated
* **Code**:

  * `TruncateStrategy` implemented
  * `TokenBudget` supports `should_compact()`
  * `SessionConfig` includes `token_budget`
  * **BUT**: `query.rs` never calls compact logic

---

### 1.4 Micro Compact ✅ Matches

* **goal.md**: Missing
* **Code**: No implementation anywhere

---

### 1.5 ContextTooLong Recovery ✅ Matches

* **goal.md**: Error type exists, no recovery
* **Code**:

  * `AgentError::ContextTooLong` exists
  * No retry or special handling in `query.rs`

---

### 1.6 MaxOutputTokens Recovery ✅ Matches

* **goal.md**: Missing
* **Code**:

  * `StopReason::MaxTokens` defined
  * Not checked in query loop

---

### 1.7 Token Budget Control ✅ Matches

* **goal.md**: Exists but unused
* **Code**:

  * Full budget logic implemented
  * Only `max_output_tokens` used
  * `should_compact()` never used

---

### 1.8 Stop Hooks ✅ Matches

* **goal.md**: Missing
* **Code**: No hook system

---

### 1.9 Memory Prefetch ✅ Matches

* **goal.md**: Missing
* **Code**: Only static system prompt, no CLAUDE.md loading

---

### 1.10 Interrupt & Resume ✅ Matches

* **goal.md**: Missing
* **Code**:

  * No Ctrl+C handler
  * `AgentError::Aborted` unused
  * Process exits directly

---

### 1.11 Usage Tracking ⚠️ Mostly correct

* **goal.md**: Partial, no cache stats
* **Code**:

  * Token counters exist
  * Cache fields exist but unused

**More accurate description**: cache fields are defined but not used/read/displayed

---

### 1.12 Unified streaming protocol ✅ Matches

* **goal.md**: Implemented
* **Code**: Unified `StreamEvent` across providers

---

## 3. Additional Findings (not in goal.md)

---

### 3.1 PermissionPolicy recreated every loop

Each loop creates a new policy instance:

```rust
permissions: Arc::new(RuleBasedPolicy::new(...))
```

This prevents persistent permission state (e.g., Allow Once / Always Allow).
Should instead be stored at session level.

---

### 3.2 No error classification in streaming

All errors are treated the same:

```rust
return Err(AgentError::Provider(e));
```

Missing distinctions:

* **429** → retry with backoff
* **5xx** → retry
* **context_too_long** → trigger compact

This is a prerequisite for:

* 1.5 Context recovery
* 10.3 Retry logic

---

## 4. Suggested Implementation Roadmap

```
Phase 1: Conversation survival
  1.10 Interrupt & Resume → 1.5 ContextTooLong Recovery → 1.3 Auto Compact
       │                           │
       │                           └── requires error classification (3.2)
       └── requires signal handling + cancellation

Phase 2: Output completeness
  1.6 MaxOutputTokens Recovery

Phase 3: Cost & efficiency
  1.7 Token Budget → 1.4 Micro Compact
       │
       └── requires Auto Compact

Phase 4: Context quality
  1.9 Memory Prefetch → 1.11 Usage tracking improvements

Phase 5: Extensibility
  1.8 Stop Hooks
```

---

### Key Dependencies

* **1.3 depends on 1.7** — without budget, compact timing is blind
* **1.5 depends on error classification (3.2)**
* **1.4 depends on 1.3** — micro compact builds on auto compact
