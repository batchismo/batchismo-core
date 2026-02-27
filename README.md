# Batchismo

A locally-installed, OS-native AI agent platform that runs persistently on your machine. Batchismo uses an **always-delegate orchestrator model** — every user message is routed through an orchestrator that delegates to specialized worker agents. Workers run in isolated processes, can ask questions back, and can be paused, resumed, cancelled, or instructed mid-flight. The result is a personal AI that does real work: executing tasks, managing files, speaking aloud, and learning how you work over time.

> **No Docker. No config files to hand-edit.** Install, connect your API key, point it at your folders, and have a working personal AI agent within minutes.

---

## What It Does

- **Orchestrator-driven architecture** — every message goes through an orchestrator that delegates to worker agents with bidirectional communication
- **Executes real tasks** on your machine within user-defined path boundaries
- **Voice input and output** — TTS (OpenAI with 10 voice choices, ElevenLabs) and STT built in; the agent knows when voice is active
- **Telegram integration** — connect a Telegram bot as an alternative interface with user allow-listing
- **Active memory reflection** — automatically updates MEMORY.md after conversations, with periodic consolidation
- **Learns your patterns** and updates its own behavior files over time
- **Runs worker agents concurrently** while you continue conversing — workers are visible in the Activity Panel with status and cancel controls
- **Enforces path policy** at the tool layer for all filesystem access
- **Full audit logging** of every agent action, tool call, and file access
- **Ships as a single installable app** — terminal experience not required (but available via `bat-tui`)

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Tauri Shell (Rust)                  │
│     Window · Tray · Auto-start · Auto-update         │
└───────────────────────┬─────────────────────────────┘
                        │ Tauri IPC
┌───────────────────────▼─────────────────────────────┐
│               Gateway Runtime (Rust)                 │
│  SessionManager · Orchestrator · EventBus · SQLite   │
│  TTS/STT · Memory Reflection · Telegram Adapter      │
└───────────────────────┬─────────────────────────────┘
                        │ Named Pipe (NDJSON)
            ┌───────────┴──────────┐
            │   Worker Agents      │  ← isolated OS process per session
            │  LLM loop · Tools    │
            │  Bidirectional comms │
            │  Path policy enforced│
            └──────────────────────┘
```

The orchestrator receives every user message and delegates to worker agents. Workers can:
- Ask clarifying questions back to the orchestrator
- Be paused, resumed, cancelled, or given new instructions mid-flight
- Run concurrently (configurable limit, default: 5)
- Post structured summaries back when complete

---

## Crate Structure

| Crate | Type | Purpose |
|---|---|---|
| `bat-types` | library | Shared types: `Message`, `SessionMeta`, `PathPolicy`, IPC envelopes |
| `bat-gateway` | library | Core gateway: session management, orchestrator, SQLite, config, event bus, IPC server, TTS/STT, memory reflection, channel adapters |
| `bat-agent` | binary | Worker agent process: LLM loop, tool execution, path policy enforcement |
| `bat-shell` | Tauri app | Desktop shell: window, tray, React UI, Tauri commands |
| `bat-tui` | binary | Terminal UI: ratatui + crossterm, same gateway, keyboard-driven |

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+ (for the UI)
- An [Anthropic API key](https://console.anthropic.com/)
- Optional: [OpenAI API key](https://platform.openai.com/) for TTS voices

---

## Getting Started

```bash
# 1. Clone the repo
git clone git@github.com:batchismo/batchismo-core.git
cd batchismo-core

# 2. Install UI dependencies
cd crates/bat-shell/ui && npm install && cd ../../..

# 3. Set your API key
export ANTHROPIC_API_KEY=<your-key>

# 4. Run in development mode
cargo tauri dev --manifest-path crates/bat-shell/Cargo.toml
```

On first launch, Batchismo creates `~/.batchismo/` with a default `config.toml` and workspace MD files.

---

## Configuration

Config lives at `~/.batchismo/config.toml`. The Settings UI manages it — you rarely need to edit it directly.

**The API key is never stored in `config.toml`.** Set it via the `ANTHROPIC_API_KEY` environment variable, or enter it through the Settings UI (stored in your OS keychain).

### Settings UI Pages

| Page | What it configures |
|---|---|
| Agent Config | Name, model, thinking level |
| API Keys | Anthropic, OpenAI keys |
| Path Policies | Folder access grants (read/write/read-write) |
| Tools | Toggle individual tools on/off |
| Voice | TTS provider, voice selection (10 OpenAI voices), STT |
| Channels | Telegram bot token, allowed user IDs |
| About | Version info |

---

## Voice / TTS + STT

Batchismo supports text-to-speech and speech-to-text natively:

- **OpenAI TTS** — 10 voices (alloy, ash, ballad, coral, echo, fable, nova, onyx, sage, shimmer), opus format
- **ElevenLabs TTS** — custom voice IDs, MP3 → OGG conversion via ffmpeg
- **STT** — speech-to-text for voice input

TTS is handled transparently by the gateway — when enabled, agent text responses are automatically synthesized to audio. The system prompt tells the agent that voice is active so it adjusts its response style accordingly.

Configure voice provider and select voices in **Settings → Voice**.

---

## Tools

### Built-in Tools

| Tool | Description |
|---|---|
| `fs_read` | Read file contents |
| `fs_write` | Write or create files |
| `fs_list` | List directory contents |
| `fs_move` | Move or rename a file |
| `fs_search` | Search for files by name or content |
| `fs_stat` | Get file metadata |
| `fs_read_pdf` | Extract text from PDF files (Anthropic-powered) |
| `web_fetch` | Fetch a URL (HTTP GET) |
| `web_search` | Search the web via configured search API |
| `exec_run` | Execute a shell command |
| `exec_list` | List running processes |
| `exec_output` | Get output from a running process |
| `exec_write` | Write to a running process's stdin |
| `exec_kill` | Kill a running process |
| `screenshot` | Capture a screenshot |
| `clipboard` | Read/write clipboard contents |
| `app_open` | Open an application |
| `system_info` | Get system information |
| `shell_run` | Run a shell command |
| `session_spawn` | Spawn a background worker agent |
| `session_status` | Check worker agent status |
| `session_cancel` | Cancel a running worker |
| `session_pause` | Pause a running worker |
| `session_resume` | Resume a paused worker |
| `session_instruct` | Send new instructions to a running worker |
| `session_answer` | Answer a question from a worker |
| `ask_orchestrator` | Worker asks the orchestrator a question |

All tools are toggleable from **Settings → Tools**.

---

## Channel Adapters

| Channel | Status |
|---|---|
| Built-in WebChat | ✅ Available |
| Telegram (Bot API) | ✅ Available |
| Discord | 🔜 Planned |

**Telegram setup:** Configure your bot token and allowed user IDs in **Settings → Channels**. Only users in the allow list can interact with the agent. Use [@userinfobot](https://t.me/userinfobot) to find your Telegram user ID.

---

## Memory System

Batchismo includes an active memory system:

- **Memory reflection** — after orchestrator turns, the agent reflects on what it learned and updates `MEMORY.md` automatically
- **Memory consolidation** — periodic consolidation of observations into higher-level patterns
- **Workspace files** — human-readable Markdown files the agent reads at session start:

| File | Purpose |
|---|---|
| `IDENTITY.md` | Agent name, role, and core constraints |
| `MEMORY.md` | Facts learned about you (auto-updated via reflection) |
| `PATTERNS.md` | Higher-level behavioral patterns |
| `SKILLS.md` | Available skills and usage notes |
| `TOOLS.md` | Tool usage preferences and constraints |

You can edit these directly. The agent reads them at the start of every session.

---

## File System Access

**The agent has no filesystem access by default.** You explicitly grant access to specific paths through the UI:

- **read-only** — agent can read but not modify files
- **read-write** — agent can read, create, and modify files
- **write-only** — agent can deposit files but not read existing content

Path policy is enforced at the tool layer for all filesystem operations.

---

## Building

```bash
# Run all tests
cargo test --workspace

# Build release binaries
cargo build --release --workspace

# Build the full desktop app (installer)
cargo tauri build --manifest-path crates/bat-shell/Cargo.toml
```

---

## Debugging & Development

### Real-time logging

Run the app with `RUST_LOG` set to see all gateway and agent output in your terminal:

```powershell
# PowerShell — dev mode with debug logging
$env:RUST_LOG="debug"
cargo tauri dev --manifest-path crates/bat-shell/Cargo.toml
```

This also auto-opens Chrome DevTools for the frontend (React/TypeScript) side.

To capture logs to a file while also viewing them:

```powershell
$env:RUST_LOG="debug"
cargo tauri dev --manifest-path crates/bat-shell/Cargo.toml 2>&1 | Tee-Object -FilePath "$env:USERPROFILE\.batchismo\debug.log"
```

### Log levels

Set `RUST_LOG` to control verbosity:

| Value | What you see |
|---|---|
| `error` | Crashes and critical failures only |
| `warn` | Warnings + errors |
| `info` | Session lifecycle, tool calls, API requests (recommended) |
| `debug` | Everything — IPC messages, config loads, path policy checks |
| `trace` | Extremely verbose — includes raw API payloads |

You can also filter by crate: `RUST_LOG="bat_gateway=debug,bat_agent=info"`

### Common debug tasks

```powershell
# Reset the database (clears all sessions/history)
Remove-Item "$env:USERPROFILE\.batchismo\batchismo.db" -Force

# Check if bat-agent.exe is running
Get-Process bat-agent -ErrorAction SilentlyContinue

# Kill orphaned agent processes
Stop-Process -Name bat-agent -Force -ErrorAction SilentlyContinue

# Build just the agent binary for quick iteration
cargo build -p bat-agent

# Run tests for a specific crate
cargo test -p bat-types
cargo test -p bat-gateway
```

### Windows path canonicalization

Windows `canonicalize()` returns paths with a `\\?\` prefix (extended-length path notation). This is handled internally — path policies defined in the UI (e.g., `C:\Users\You\Documents`) will match canonicalized paths correctly. If you see "Access denied" errors despite having a policy configured, check that the policy path and target path resolve to the same location after prefix stripping.

---

## Project Status

Batchismo is in active development. Current version: **v0.3.6**

| Phase | Goal | Status |
|---|---|---|
| 1 | Core agent loop: chat, fs tools, SQLite, streaming | ✅ Complete |
| 2 | Orchestrator model, bidirectional worker agents, Telegram | ✅ Complete |
| 3 | Memory reflection, consolidation, TTS/STT, voice selection | ✅ Complete |
| 4 | Installer, onboarding wizard, OS-native sandboxing, audit log UI | 🔜 Planned |
| 5 | Skill system, Discord, metrics dashboard | 🔜 Planned |

---

## License

Private — Nightfall Advisors. All rights reserved.
