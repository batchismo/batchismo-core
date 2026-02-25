# Task: Implement Orchestrator Model (All Phases)

Read `project/PRD.md` Section 26 for the full spec. Read `CLAUDE.md` for project conventions.

Work on the `dev` branch. Commit frequently with clear messages.

## Overview

Transform Batchismo so the main chat session acts as an **orchestrator** that delegates ALL work to sub-agents. The user's chat stays responsive at all times. Sub-agents do the actual work. Sub-agents can ask questions back, and the orchestrator answers them (or escalates to the user).

## What to implement (in order)

### Phase A: Orchestrator Prompt + Always-Delegate

1. **Split system prompts** in `crates/bat-gateway/src/system_prompt.rs`:
   - `build_orchestrator_prompt()` — for main/user sessions. Lists ONLY session management tools (session_spawn, session_status, session_pause, session_resume, session_instruct, session_cancel, session_answer). Instructs the agent to always delegate work. Should still include memory, patterns, skills, identity sections.
   - `build_worker_prompt()` — for sub-agent sessions. Lists ALL action tools (fs_read, fs_write, fs_list, web_fetch, shell_run, exec_*, app_open, system_info, clipboard, screenshot). Does NOT include session_spawn (sub-agents can't spawn sub-agents). Includes the task description prominently.
   - The gateway chooses which prompt based on `SessionKind` (main vs subagent).

2. **Update tool registration** in `crates/bat-agent/src/tools/mod.rs`:
   - Add a `with_orchestrator_tools()` method that only registers session management tools.
   - The existing `with_default_tools()` becomes the worker tool set.

3. **Pass session kind to bat-agent** via the `Init` IPC message. Add `session_kind: String` field to `GatewayToAgent::Init` in `crates/bat-types/src/ipc.rs`. The agent uses this to decide which tool registry to build.

### Phase B: Bidirectional Communication

4. **New IPC message types** in `crates/bat-types/src/ipc.rs`:
   ```rust
   // Agent → Gateway
   AgentToGateway::Question {
       question_id: String,
       question: String,
       context: String,
       blocking: bool,  // if true, agent waits for answer before continuing
   }
   AgentToGateway::Progress {
       summary: String,
       percent: Option<f32>,
   }

   // Gateway → Agent  
   GatewayToAgent::Answer {
       question_id: String,
       answer: String,
   }
   GatewayToAgent::Instruction {
       instruction_id: String,
       content: String,
   }
   ```

5. **Sub-agent tools for asking questions** in `crates/bat-agent/src/tools/`:
   - `ask_orchestrator.rs` — tool that sub-agents use to ask questions. Sends `AgentToGateway::Question` via the gateway bridge. If `blocking: true`, waits for the answer.
   - Register this tool in the worker tool set (not orchestrator).

6. **Gateway message router** in `crates/bat-gateway/src/lib.rs`:
   - When a sub-agent sends a `Question`, route it to the parent session
   - The parent session's orchestrator sees it as a special system message
   - When the orchestrator calls `session_answer`, route the answer back to the waiting sub-agent
   - Track parent→child relationships: `SubagentInfo` already has this, extend if needed

7. **Orchestrator tool: `session_answer`** in `crates/bat-agent/src/tools/`:
   - Takes `session_key` and `answer` as input
   - Sends the answer via gateway bridge to the waiting sub-agent

### Phase C: Lifecycle Management

8. **New orchestrator tools** in `crates/bat-agent/src/tools/`:
   - `session_pause.rs` — pauses a sub-agent (sets state, agent checks between steps)
   - `session_resume.rs` — resumes a paused sub-agent, optionally with new instructions
   - `session_instruct.rs` — sends new instructions to a running sub-agent
   - `session_cancel.rs` — cancels a sub-agent and cleans up

9. **Sub-agent state machine** in `crates/bat-types/src/ipc.rs` or a new file:
   ```rust
   pub enum SubagentState {
       Running,
       WaitingForAnswer,
       Paused,
       Completed,
       Failed,
       Cancelled,
   }
   ```
   Update `SubagentInfo` in `crates/bat-gateway/src/lib.rs` to track state.

10. **Mid-turn message checking** in `crates/bat-agent/src/agent_loop.rs`:
    - Between LLM calls and tool executions, check for incoming messages (pause, instructions)
    - If paused, enter a wait loop until resumed or cancelled
    - If new instructions arrive, inject them as context for the next LLM call

### Phase D: UI Updates

11. **Desktop UI** in `crates/bat-shell/ui/src/`:
    - `StatusBar.tsx`: Show active sub-agent count
    - `ActivityPanel.tsx`: Show sub-agent states, progress, pending questions
    - `ChatPanel.tsx`: Render sub-agent questions as special message bubbles, completions as summary cards

12. **TUI** in `crates/bat-tui/src/ui/`:
    - `chat.rs`: Render sub-agent questions with `[?]` prefix
    - `activity.rs`: Show sub-agent states and progress
    - Status bar: active sub-agent count

13. **Tauri commands** in `crates/bat-shell/src/commands.rs`:
    - Add commands for any new gateway methods needed by the UI

## Architecture Guidelines

- **Simple always beats complex.** If you can solve something with a string match instead of a trait hierarchy, do it.
- **Reuse existing code.** The gateway bridge pattern (`GatewayBridge` + `BridgePending`) already handles sync tool→async IPC. Extend it for new message types.
- **Async when possible.** Use tokio throughout. For sync→async bridges, use `tokio::task::block_in_place(|| Handle::current().block_on(...))`.
- **Logging everywhere.** Use `tracing::info!` / `tracing::warn!` / `tracing::error!` for all significant events.
- **Test cases.** Add unit tests for new types, serialization, state transitions. Tests go in the same file or a `tests` module.
- **Zero warnings.** `cargo check` must produce zero warnings when done.
- **PowerShell note:** This is a Windows dev machine. No `&&` in shell — tests run fine via `cargo test`.

## Key Files Reference

- `crates/bat-types/src/ipc.rs` — IPC message types (GatewayToAgent, AgentToGateway)
- `crates/bat-types/src/lib.rs` — re-exports
- `crates/bat-gateway/src/lib.rs` — Gateway struct, session management, sub-agent spawning
- `crates/bat-gateway/src/system_prompt.rs` — system prompt builder
- `crates/bat-gateway/src/ipc.rs` — pipe server, agent spawning
- `crates/bat-agent/src/main.rs` — agent entry point, Init handling
- `crates/bat-agent/src/agent_loop.rs` — main agent turn loop
- `crates/bat-agent/src/tools/mod.rs` — tool registry
- `crates/bat-agent/src/gateway_bridge.rs` — sync→async bridge for tools
- `crates/bat-shell/ui/src/components/` — React UI components
- `crates/bat-tui/src/ui/` — TUI screens

## Existing Types to Know About

- `SubagentInfo` / `SubagentStatus` in `crates/bat-gateway/src/lib.rs`
- `SessionKind` enum in `crates/bat-types/src/ipc.rs` (or config.rs — check both)
- `GatewayBridge` + `BridgePending` in `crates/bat-agent/src/gateway_bridge.rs`
- `ProcessManager` in `crates/bat-gateway/src/process_manager.rs`

## When Done

Run `cargo test` and `cargo check` — zero errors, zero warnings. Commit all changes to `dev` branch.
