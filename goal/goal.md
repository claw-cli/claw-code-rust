## 1. Core Runtime (Query Runtime)

| #    | Capability                 | Claude Code Behavior                                                                                | rust-clw Status                                         |
| ---- | -------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| 1.1  | Multi-turn loop            | Model output → detect tool_use → execute tool → feed back tool_result → continue until no tool call | **Implemented**                                         |
| 1.2  | Streaming output           | Token-by-token streaming, rendered in real time in terminal                                         | **Implemented**                                         |
| 1.3  | Auto Compact               | Automatically compress history when token usage exceeds threshold (autoCompact)                     | Compact crate exists, **not integrated into main loop** |
| 1.4  | Micro Compact              | Compress only overly long tool_result entries instead of full history                               | **Missing**                                             |
| 1.5  | ContextTooLong recovery    | On API context-too-long error, perform reactiveCompact and retry                                    | Error type exists, **no recovery logic**                |
| 1.6  | MaxOutputTokens recovery   | If output is truncated, append “please continue” to resume                                          | **Missing**                                             |
| 1.7  | Token budget control       | Compute input_budget each round to decide compression                                               | Field exists, **not enforced**                          |
| 1.8  | Stop hooks                 | Run hooks after each round to decide whether to stop                                                | **Missing**                                             |
| 1.9  | Memory prefetch            | Load memory files (e.g., CLAUDE.md) before first query                                              | **Missing**                                             |
| 1.10 | Interrupt & resume         | Ctrl+C interrupts generation but preserves context                                                  | **Missing** (currently exits process)                   |
| 1.11 | Usage tracking             | Track input/output tokens and cache usage per round                                                 | Partial (tokens only), **no cache stats**               |
| 1.12 | Unified streaming protocol | Normalize Anthropic/OpenAI/Ollama streams into StreamEvent                                          | **Implemented**                                         |

---

## 2. Tools

### 2.1 Improvements to existing tools

| #     | Capability                   | Claude Code Behavior                            | rust-clw Status                  |
| ----- | ---------------------------- | ----------------------------------------------- | -------------------------------- |
| 2.1.1 | Bash timeout                 | Supports `timeout_ms`, auto-kills on timeout    | **Implemented**                  |
| 2.1.2 | Bash background execution    | Long commands run as background tasks           | **Missing** (all blocking)       |
| 2.1.3 | Bash output truncation       | Truncate long output, keep head & tail          | **Missing**                      |
| 2.1.4 | FileEdit replace_all         | Replace all matches                             | **Missing** (only `replacen(1)`) |
| 2.1.5 | FileEdit diagnostics         | Provide candidates when match is not unique     | **Missing**                      |
| 2.1.6 | FileRead large file handling | Truncate large files and suggest offset/limit   | **Missing**                      |
| 2.1.7 | Grep formatting              | Group by file, show line numbers, context lines | **No context support**           |

### 2.2 Missing tools

| #      | Tool             | Claude Code Behavior                         | Priority |
| ------ | ---------------- | -------------------------------------------- | -------- |
| 2.2.1  | WebFetch         | HTTP GET + convert to readable markdown      | P0       |
| 2.2.2  | WebSearch        | Call search API and return summaries/links   | P0       |
| 2.2.3  | AgentTool        | Spawn sub-agent with independent loop        | P0       |
| 2.2.4  | TodoWrite        | Structured task list with status transitions | P1       |
| 2.2.5  | NotebookEdit     | Edit Jupyter notebook cells by index         | P1       |
| 2.2.6  | MCPTool          | Call MCP server tools                        | P1       |
| 2.2.7  | ListMcpResources | List MCP resources                           | P1       |
| 2.2.8  | ReadMcpResource  | Read MCP resource content                    | P1       |
| 2.2.9  | TaskCreate       | Create background task                       | P1       |
| 2.2.10 | TaskGet          | Query task status/output                     | P1       |
| 2.2.11 | TaskList         | List tasks                                   | P1       |
| 2.2.12 | TaskStop         | Stop task                                    | P1       |
| 2.2.13 | PowerShell       | Windows shell execution                      | P2       |
| 2.2.14 | LSPTool          | Get diagnostics (linter errors)              | P2       |
| 2.2.15 | SkillTool        | Execute Skill files                          | P2       |

---

## 3. Permissions System

| #   | Capability          | Claude Code Behavior                   | rust-clw Status             |
| --- | ------------------- | -------------------------------------- | --------------------------- |
| 3.1 | Auto mode           | Auto-allow read/write/execute          | **Implemented**             |
| 3.2 | Deny mode           | Reject all non-read-only tools         | **Implemented**             |
| 3.3 | Interactive confirm | Prompt user Y/N                        | **Missing**                 |
| 3.4 | Allow once          | Allow only this time                   | **Missing**                 |
| 3.5 | Always allow        | Persist allow in session               | **Missing**                 |
| 3.6 | Persist rules       | Save to config file                    | **Missing**                 |
| 3.7 | Centralized checks  | Permission checks only in orchestrator | **Duplicated checks exist** |

---

## 4. Context Compression (Compact)

| #   | Capability       | Claude Code Behavior                             | rust-clw Status       |
| --- | ---------------- | ------------------------------------------------ | --------------------- |
| 4.1 | TruncateStrategy | Drop oldest messages first                       | Exists, **not wired** |
| 4.2 | SummaryStrategy  | Summarize history via model                      | **Missing**           |
| 4.3 | autoCompact      | Trigger based on token count                     | **Missing**           |
| 4.4 | microCompact     | Compress long tool_result only                   | **Missing**           |
| 4.5 | reactiveCompact  | Compress after context error                     | **Missing**           |
| 4.6 | Session memory   | Extract key info into memory                     | **Missing**           |
| 4.7 | Context collapse | Replace long output with summary + expand marker | **Missing**           |

---

## 5. Background Task System

| #   | Capability            | Claude Code Behavior                    | rust-clw Status                     |
| --- | --------------------- | --------------------------------------- | ----------------------------------- |
| 5.1 | TaskManager lifecycle | Manage states: running/completed/failed | Exists, **not integrated**          |
| 5.2 | LocalShellTask        | Background shell execution              | **Missing**                         |
| 5.3 | LocalAgentTask        | Background sub-agent                    | **Missing**                         |
| 5.4 | Result injection      | Inject results into main conversation   | **Missing**                         |
| 5.5 | Visibility            | Show tasks in UI                        | **Missing**                         |
| 5.6 | Cancellation          | Stop running tasks                      | Trait exists, **no implementation** |

---

## 6. MCP Integration

| #   | Capability         | Claude Code Behavior         | rust-clw Status |
| --- | ------------------ | ---------------------------- | --------------- |
| 6.1 | MCP client         | stdio communication          | **Missing**     |
| 6.2 | Config loading     | Load from config file        | **Missing**     |
| 6.3 | Auto-start servers | Spawn MCP servers            | **Missing**     |
| 6.4 | Tool discovery     | tools/list                   | **Missing**     |
| 6.5 | Tool registration  | Inject into registry         | **Missing**     |
| 6.6 | Tool execution     | Call MCP tools               | **Missing**     |
| 6.7 | Resource browsing  | resources/list/read          | **Missing**     |
| 6.8 | Connection mgmt    | Heartbeat/reconnect          | **Missing**     |
| 6.9 | Multi-server       | Support multiple MCP servers | **Missing**     |

---

## 7. Session Management

| #   | Capability     | Claude Code Behavior     | rust-clw Status |
| --- | -------------- | ------------------------ | --------------- |
| 7.1 | Persistence    | Save sessions to disk    | **Missing**     |
| 7.2 | Resume         | Restore previous session | **Missing**     |
| 7.3 | Session list   | Browse past sessions     | **Missing**     |
| 7.4 | Session memory | Extract key info         | **Missing**     |
| 7.5 | Session ID     | Unique ID per session    | **Exists**      |
| 7.6 | CWD tracking   | Track working directory  | **Exists**      |

---

## 8. Command System (Slash Commands)

All commands such as `/help`, `/compact`, `/clear`, `/cost`, `/model`, `/permissions`, `/memory`, `/exit`, `/config`, `/resume`, `/diff` are **missing**.

---

## 9. REPL Experience

All interactive features (line editing, history, tab completion, markdown rendering, spinner, diff view, interrupt handling, status bar, permission UI) are **missing**.

---

## 10. Provider & API

| #    | Capability           | Claude Code Behavior | rust-clw Status |
| ---- | -------------------- | -------------------- | --------------- |
| 10.1 | Anthropic streaming  | Full support         | **Implemented** |
| 10.2 | OpenAI compatibility | Supported            | **Implemented** |
| 10.3 | Retry logic          | Retry on 429/5xx     | **Missing**     |
| 10.4 | Prompt cache         | Reduce cost          | **Missing**     |
| 10.5 | Model fallback       | Switch on failure    | **Missing**     |
| 10.6 | Cost calculation     | Compute usage cost   | **Missing**     |

---

## Summary

| Module          | Total  | Done   | Partial | Missing |
| --------------- | ------ | ------ | ------- | ------- |
| Core runtime    | 12     | 3      | 1       | 8       |
| Tools (improve) | 7      | 1      | 0       | 6       |
| Tools (missing) | 15     | 0      | 0       | 15      |
| Permissions     | 7      | 2      | 0       | 5       |
| Compact         | 7      | 0      | 1       | 6       |
| Tasks           | 6      | 0      | 2       | 4       |
| MCP             | 9      | 0      | 1       | 8       |
| Session         | 6      | 2      | 0       | 4       |
| Commands        | 12     | 0      | 0       | 12      |
| REPL            | 10     | 0      | 0       | 10      |
| Provider        | 6      | 2      | 0       | 4       |
| **Total**       | **97** | **10** | **5**   | **82**  |

Current alignment: **~15% (15/97)**
Remaining work: **82 capabilities**
