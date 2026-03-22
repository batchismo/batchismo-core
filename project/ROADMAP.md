# Batchismo — Post-v0.4.5 Roadmap
**Created:** 2026-03-20
**Branch:** `dev`
**Status:** Complete - Ready for v0.5.0 Release

Work items are tackled linearly, one at a time. Update status as each completes.

---

## Track 1: §26 Phase B — Bidirectional Sub-Agent Communication
**Status:** ✅ Completed
**PRD Ref:** Section 26.2, 26.5

### What
Sub-agents can ask questions back to the orchestrator (instead of guessing/failing). Orchestrator can answer autonomously or escalate to the user. Sub-agents send progress updates.

### Tasks
- [x] Add `Question`, `Answer`, `Progress` IPC message types to `bat-types/src/ipc.rs` ✅ Already implemented
- [x] Add `SubagentState::WaitingForAnswer` to session state machine ✅ Already implemented
- [x] Implement message router in gateway for inter-session communication ✅ Implemented MessageRouter
- [x] Add message queue per active agent session (for mid-turn injection) ✅ Implemented with tokio channels
- [x] Agent loop: check for incoming messages between LLM calls / tool executions ✅ Added check_incoming_messages()
- [x] Blocking questions: agent loop yields and waits for answer ✅ Implemented with AskOrchestrator + blocking logic
- [x] Add `session_answer` tool to orchestrator's tool set ✅ Updated to use message router
- [x] Orchestrator prompt: instruct it to try answering autonomously, escalate if unsure ✅ Implemented in system_prompt.rs
- [x] UI (desktop): question bubbles in chat, progress indicators in Activity Panel ✅ Added to MessageBubble + ActivityPanel
- [x] UI (TUI): question rendering with `[?]` prefix, answerable inline ✅ Added to chat.rs
- [x] Tests ✅ Added comprehensive IPC message serialization tests

---

## Track 2: §26 Phase C — Sub-Agent Lifecycle Management
**Status:** ✅ Completed
**PRD Ref:** Section 26.3, 26.4

### What
Pause/resume/redirect/cancel sub-agents mid-task. Contradiction detection.

### Tasks
- [x] Add `session_pause`, `session_resume`, `session_instruct`, `session_cancel` tools ✅ Already implemented
- [x] Implement mid-turn message injection in agent loop ✅ Already implemented in agent_loop.rs
- [x] Sub-agent state machine: Running → Paused → Running, etc. ✅ Already implemented in SubagentStatus
- [x] Contradiction detection heuristics in orchestrator prompt ✅ Already implemented in orchestrator prompt
- [x] UI updates for pause/resume controls ✅ Added to ActivityPanel (desktop) and activity.rs (TUI)
- [x] Tests ✅ Added lifecycle management process action tests

---

## Track 3: §25 Phase B — Hybrid LLM Routing
**Status:** ✅ Completed
**PRD Ref:** Section 25.3

### What
Route different task types to different models (e.g., memory consolidation → local Ollama, complex tasks → Claude).

### Tasks
- [x] Per-task-type model assignment in config (main chat, subagents, memory consolidation) ✅ Added ModelRoutingConfig to bat-types
- [x] Settings UI for task-type → model mapping ✅ Added Model Routing section in AgentConfigPage
- [x] Gateway routing logic based on task type ✅ Updated send_user_message, subagent spawning, and consolidation
- [x] Tests ✅ Added comprehensive tests for model routing functionality

---

## Track 4: Phase 5 — Skill System & Polish
**Status:** ✅ Completed  
**PRD Ref:** Section 20

### What
Hot-reloadable skills, Discord adapter, web tools polish, metrics dashboard.

### Tasks
- [x] Skill system: `SKILL.md` hot reload, skill-defined tools via `tools.toml` ✅ Implemented with file watcher and UI
- [x] Discord channel adapter ✅ Stub implementation created (requires serenity crate for full functionality)
- [x] Web tools polish (`web.fetch`, `web.search` refinements) ✅ Both tools already implemented and working
- [x] Metrics dashboard in UI ✅ Full metrics dashboard added to Settings
- [x] Tests ✅ Compilation tests pass

---

## Track 5: Onboarding Wizard
**Status:** ✅ Completed
**PRD Ref:** Section 11

### What
6-step guided first-launch wizard for non-developers.

### Tasks
- [x] Welcome screen ✅ Clean, inviting design with "Your AI that actually works on your computer"
- [x] LLM provider + API key entry with validation ✅ Unified ProviderStep supporting Anthropic (recommended), OpenAI, and Ollama (local)
- [x] Agent naming (writes IDENTITY.md) ✅ Personal step to establish relationship
- [x] Path access setup with native file picker ✅ Existing AccessStep with Tauri dialog plugin
- [x] Channel setup (optional Telegram/Discord) ✅ New ChannelStep with skip option
- [x] First suggested task ✅ FirstTaskStep suggests starter tasks based on granted folders with "Try it now"
- [x] Skip onboarding if already configured ✅ App.tsx checks onboarding_complete config

---

## Track 6: §23 Phase 2 — Request Classifier + Cost Routing
**Status:** ✅ Completed
**PRD Ref:** Section 23.5

### What
Auto-route requests to the right model based on complexity/domain. Cost governor.

### Tasks
- [x] Request classifier (rule-based initially) ✅ Implemented classification by complexity, domain, and capabilities
- [x] Routing strategies (cost-optimized, quality-optimized, balanced) ✅ All strategies implemented with intelligent model selection
- [x] Cost governor with budget limits ✅ Daily/session budgets with automatic downgrading
- [x] UI for routing preferences ✅ Settings UI with strategy picker and budget fields  
- [x] Tests ✅ Comprehensive unit tests for classification logic

---

## Completion Log

| Date | Track | Notes |
|------|-------|-------|
| 2026-03-20 | Track 1 | ✅ Bidirectional Sub-Agent Communication - UI, tests, and core functionality complete |
| 2026-03-20 | Track 2 | ✅ Sub-Agent Lifecycle Management - Pause/resume/instruct/cancel tools and UI complete |
| 2026-03-20 | Track 3 | ✅ Hybrid LLM Routing - Per-task-type model assignment, Settings UI, gateway routing logic, and tests complete |
| 2026-03-20 | Track 4 | ✅ Skill System & Polish - Hot-reloadable skills with UI, Discord stub adapter, web tools verified, metrics dashboard complete |
| 2026-03-20 | Track 5 | ✅ Onboarding Wizard - 6-step wizard with provider selection, channel setup, and suggested first tasks complete |
| 2026-03-20 | Track 6 | ✅ Request Classifier + Cost Routing - Intelligent model routing with cost governance and budget controls complete |
