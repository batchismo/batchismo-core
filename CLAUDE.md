# Batchismo

Windows desktop AI agent. Tauri v2 shell, Rust backend, React frontend. The agent calls Anthropic's API and reads/writes files on the user's machine within user-defined path policies.

Phase 1 complete. Core agent loop, streaming chat, 3 filesystem tools, SQLite, desktop shell all working.

## Key Directories

```
crates/
  bat-types/       — shared types only (no async, no I/O, no business logic)
  bat-gateway/     — library: session mgmt, SQLite, config, IPC server, event bus
  bat-agent/       — binary: spawned per turn, runs LLM loop + tool execution
  bat-shell/       — Tauri v2 app, React UI lives in bat-shell/ui/
project/           — planning docs, specs
```

Runtime data: `~/.batchismo/` (config.toml, batchismo.db, workspace/*.md files)

## Common Commands

```bash
# Dev mode (hot-reload frontend + Rust rebuild)
cargo tauri dev

# Build agent binary only (fast iteration when changing agent logic)
cargo build -p bat-agent

# Frontend deps (first time only)
cd crates/bat-shell/ui && npm install && cd ../../..

# Tests
cargo test                  # all
cargo test -p bat-types     # 6 tests: path policy logic
cargo test -p bat-gateway   # 7 tests: DB, sessions, tokens, policies

# Production build
cargo tauri build
```

Requires `ANTHROPIC_API_KEY` in env. PowerShell: `$env:ANTHROPIC_API_KEY = "sk-ant-..."`

## Coding Conventions

- **Prefer extending existing patterns over inventing new ones.** Before creating a new module, struct, or abstraction, check if there's already something similar in the crate and extend it.
- **Match the existing style** in whatever file you're editing. If the file uses `thiserror` enums, don't switch to `anyhow` strings. If it uses `tracing::info!`, don't use `println!`.
- **Keep `bat-types` pure.** No async, no I/O, no network calls, no file system access. It's shared types and logic only. If you need I/O, it belongs in `bat-gateway` or `bat-agent`.
- **Use `anyhow::Result` for application code** (agent, gateway, shell). Use `thiserror` for library error types in `bat-types`.
- **Async by default** in gateway and agent code. Don't introduce `std::thread` or blocking calls in async contexts. If you must call blocking code, use `tokio::task::spawn_blocking`.
- **Serde derives go on everything** that crosses a boundary (IPC, DB, Tauri commands, config). Use `#[serde(rename_all = "camelCase")]` for types the frontend consumes.
- **Use `tracing`** for all logging. Never `println!` or `eprintln!` in library/production code.
- Frontend: React 18, Vite 5, Tailwind 3. Component-level useState + custom hooks. No Redux, no Zustand.
- Frontend types in `ui/src/types.ts` must mirror the Rust types they deserialize. If you change a Rust struct that the frontend consumes, update `types.ts` too.

## Architecture Rules

- **Process-per-turn:** Each user message spawns a fresh `bat-agent.exe`. No persistent agent process. The gateway sends full conversation history in the `Init` IPC message every turn.
- **IPC is NDJSON over Windows named pipes** (`\\.\pipe\bat-agent-{session_id}`). Protocol enums are `GatewayToAgent` and `AgentToGateway` in `bat-types/src/ipc.rs`.
- **Event flow:** agent → pipe → gateway → EventBus (tokio broadcast) → bat-shell event forwarder → Tauri `emit("bat-event")` → React `listen("bat-event")`. Don't bypass this chain.
- **Single session** right now — key is `"main"`. Multi-session support is not built yet; don't add session routing without a spec.
- **Tool execution is synchronous** inside the agent's tokio task. This is a known limitation. Don't add new tools that do long-running I/O without wrapping in `spawn_blocking`.
- **Agent binary must be co-located** with `bat-shell.exe` at runtime. `spawn_agent()` looks for it in the same directory.

## Path Policy

All filesystem tools enforce path policies from the DB. Key details:

- `check_access(policies, target, write)` in `bat-types/src/policy.rs`
- Policies are stored as-entered (NOT canonicalized). The Browse button returns correct paths; manual entry may not match.
- `strip_win_prefix()` handles `\\?\` but not case differences — this is a known gap.
- When adding new filesystem tools, always check policy before any I/O. Follow the pattern in `fs_read.rs`.

## IPC Protocol Flow

```
Gateway → Agent:  Init { session_id, model, system_prompt, history, path_policies, disabled_tools }
Gateway → Agent:  UserMessage { content }
Agent → Gateway:  TextDelta { content }        (streaming, first LLM call only)
Agent → Gateway:  ToolCallStart { ... }
Agent → Gateway:  ToolCallResult { ... }
Agent → Gateway:  TurnComplete { message }     (agent exits after this)
```

Max 10 tool iterations per turn. First LLM call streams (SSE); subsequent calls after tool use are non-streaming. `Cancel` exists in the protocol but is not wired up yet.

## Model Name

Config may store `"anthropic/claude-opus-4-6"`. The `anthropic/` prefix is stripped in `bat-agent/src/main.rs` before API calls. Always use the bare model name when hitting the API.

## Gotchas

1. **Large histories will eventually break.** Full history is serialized into the Init IPC message every turn. No truncation or summarization yet.
2. **EventBus buffer overflow drops events silently** with a tracing warning. If streaming text appears incomplete, this may be why.
3. **`GatewayToAgent::Cancel` is dead code.** It's defined but never sent. Don't rely on it working.
4. **No tests for bat-agent or bat-shell yet.** When adding features to these crates, add tests.
5. **`max_tokens` is hardcoded to 4096** in `agent_loop.rs`. Not configurable yet.
6. **API version header is `2023-06-01`** — check if this needs updating when adding new Anthropic API features.

## Adding a New Tool

1. Create `crates/bat-agent/src/tools/your_tool.rs`
2. Implement the `Tool` trait (see `fs_read.rs` for the pattern)
3. Check path policy if the tool touches the filesystem
4. Register in `ToolRegistry` in `tools/mod.rs`
5. Add the tool name to the `disabled_tools` config handling if it should be toggleable
6. Update `ui/src/types.ts` if the frontend needs to know about it
7. Add tests

## Voice / TTS

The gateway handles TTS transparently — when TTS is enabled, agent text responses are synthesized to audio and sent alongside text. The system prompts tell the agent about this so it doesn't claim to be text-only.

- OpenAI TTS: uses configured voice (default "alloy"), opus format
- ElevenLabs: uses configured voice ID, MP3 → OGG conversion via ffmpeg
- Voice config is in `VoiceConfig` (`bat-types/src/config.rs`)
- TTS synthesis is in `bat-gateway/src/tts.rs`
- Gateway wires it in `lib.rs` after agent response

When a user provides an OpenAI API key, they should be able to select from available OpenAI voices in the Settings UI.

## What's Not Built Yet

See README.md for the full roadmap. Don't implement these without a spec:
- Memory system (MEMORY.md, PATTERNS.md auto-updates)
- Subagents / concurrent sessions
- OS-native sandboxing (Job Objects, seccomp, Seatbelt)
- Web tools, process.run, skill system
- Channel adapters (Telegram, Discord)
- Onboarding wizard, tray icon, auto-update