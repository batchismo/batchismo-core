# Batchismo

A locally-installed, OS-native AI agent platform that runs persistently on your machine. Batchismo acts as a personal AI that does real work; executing tasks, managing files, and coordinating parallel subagents. While continuously learning how you work over time.

> **No Docker. No config files to hand-edit.** Install, connect your API key, point it at your folders, and have a working personal AI agent within minutes.

---

## What It Does

- **Executes real tasks** on your machine within user-defined path boundaries
- **Learns your patterns** and updates its own behavior files over time (MEMORY.md, PATTERNS.md)
- **Runs subagents concurrently** while you continue conversing with the main agent
- **Enforces OS-native process isolation** per agent session (no Docker required)
- **Provides full audit logging** of every agent action, tool call, and file access
- **Ships as a single installable app** terminal experience not required (but available)

---

## How To Use

Download the installer for your OS and run it. It will guide you through the setup process.

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
│  SessionManager · EventBus · SQLite · Config         │
└───────────────────────┬─────────────────────────────┘
                        │ Named Pipe (NDJSON)
            ┌───────────┴──────────┐
            │     bat-agent        │  ← isolated OS process per session
            │  LLM loop · Tools    │
            │  Path policy enforced│
            └──────────────────────┘
```

All components compile into a single Tauri binary. No external daemon. No separate server to manage.

---

## Crate Structure

| Crate | Type | Purpose |
|---|---|---|
| `bat-types` | library | Shared types: `Message`, `SessionMeta`, `PathPolicy`, IPC envelopes |
| `bat-gateway` | library | Core gateway: session management, SQLite, config, event bus, IPC server |
| `bat-agent` | binary | Agent process: LLM loop, tool execution, path policy enforcement |
| `bat-shell` | Tauri app | Desktop shell: window, tray, React UI, Tauri commands |
| `bat-tui` | binary | Terminal UI: ratatui + crossterm, same gateway, keyboard-driven |

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+ (for the UI)
- An [Anthropic API key](https://console.anthropic.com/)

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

Config lives at `~/.batchismo/config.toml`. The UI manages it — you rarely need to edit it directly.

```toml
[agent]
name = "Aria"
model = "anthropic/claude-opus-4-6"
thinking_level = "medium"              # off | low | medium | high

[gateway]
port = 19000                           # localhost only, never exposed externally
log_level = "info"

[memory]
update_mode = "auto"                   # auto | review | manual
consolidation_schedule = "daily"       # daily | weekly | manual
max_memory_file_size_kb = 512

[sandbox]
memory_limit_mb = 512                  # per agent process
cpu_shares = 512                       # relative weight
max_concurrent_subagents = 5

[[paths]]
path = "~/Documents/"
access = "read-write"
recursive = true
description = "Main documents"

[[paths]]
path = "~/Downloads/"
access = "read-only"
recursive = false
description = "Downloads staging"

[channels.telegram]
enabled = false
bot_token = ""                         # stored in keychain, this field is a reference

[channels.discord]
enabled = false
token = ""
```

**The API key is never stored in `config.toml`.** Set it via the `ANTHROPIC_API_KEY` environment variable, or enter it through the Settings UI (stored in your OS keychain).

---

## File and Directory Structure

```
~/.batchismo/
├── config.toml                    ← main configuration
├── Batchismo.db                   ← SQLite: sessions, transcripts, observations
│
├── workspace/
│   ├── IDENTITY.md                ← agent identity and persona
│   ├── MEMORY.md                  ← learned facts about the user (auto-updated)
│   ├── PATTERNS.md                ← observed behavioral patterns (auto-updated)
│   ├── SKILLS.md                  ← skill index and usage notes
│   ├── TOOLS.md                   ← tool preferences and constraints
│   │
│   └── skills/
│       ├── files/
│       │   └── SKILL.md
│       ├── email/
│       │   └── SKILL.md
│       └── calendar/
│           └── SKILL.md
│
├── logs/
│   ├── gateway.log                ← gateway events
│   ├── audit.log                  ← all agent actions (append-only)
│   └── agents/
│       └── <session-key>.log      ← per-session logs
│
└── workspace/.history/
    ├── MEMORY.md.2026-02-19       ← rolling history of MD file changes
    └── PATTERNS.md.2026-02-19
```

---

## Workspace Files

The agent's behavior is controlled by human-readable Markdown files in `~/.batchismo/workspace/`:

| File | Purpose |
|---|---|
| `IDENTITY.md` | Agent name, role, and core constraints |
| `MEMORY.md` | Facts learned about you (auto-updated) |
| `PATTERNS.md` | Higher-level behavioral patterns (auto-updated weekly) |
| `SKILLS.md` | Available skills and usage notes |
| `TOOLS.md` | Tool usage preferences and constraints |

You can edit these directly. The agent reads them at the start of every session.

---

## File System Access

**The agent has no filesystem access by default.** You explicitly grant access to specific paths through the UI:

- **read-only** — agent can read but not modify files
- **read-write** — agent can read, create, and modify files
- **write-only** — agent can deposit files but not read existing content

Path policy is enforced at two levels: OS-native sandbox (kernel level) and tool-layer validation.

---

## Tools

### Built-in Tools

| Tool | Description |
|---|---|
| `fs.read` | Read file contents |
| `fs.write` | Write or create files |
| `fs.list` | List directory contents |
| `fs.move` | Move or rename a file (within allowed paths) |
| `fs.search` | Search for files by name or content |
| `fs.stat` | Get file metadata |
| `web.fetch` | Fetch a URL (HTTP GET, no auth forwarding) |
| `web.search` | Search the web via configured search API |
| `process.run` | Execute a shell command (disabled by default, opt-in in Settings) |
| `memory.read` | Read current MEMORY.md content |
| `memory.propose_update` | Propose a change to MEMORY.md |
| `session.spawn` | Spawn a background subagent with a task |
| `session.list` | List active sessions |
| `session.history` | Read the transcript of a session |

All tools are toggleable from the Settings UI. Skills can also define additional tools via `tools.toml`.

---

## Subagents

The main agent can spawn background subagents using `session.spawn`. Subagents:

- Run concurrently in isolated OS processes — the main agent and UI remain fully responsive
- Inherit (but cannot expand) the parent's path policy
- Cannot spawn their own subagents
- Post a structured summary back to the main session when they complete
- Are visible in the Activity Panel with elapsed time, status, and a cancel button

Maximum concurrent subagents is configurable (default: 5).

---

## Channel Adapters

In addition to the built-in WebChat UI, Batchismo can receive messages from external platforms:

| Channel | Status |
|---|---|
| Built-in WebChat | Available |
| Telegram (Bot API) | Phase 2 |
| Discord | Phase 5 |
| Slack, WhatsApp, iMessage | Planned (v2) |

Each channel enforces an `allow_from` list — only permitted users can interact with the agent. Unknown senders receive no response.

---

## Onboarding

The first-launch experience is a guided wizard — no documentation reading required:

1. **Welcome** — brief explanation of what Batchismo does
2. **LLM Provider** — choose provider, enter API key (validated immediately, stored in OS keychain)
3. **Name your agent** — sets `IDENTITY.md` and establishes the relationship
4. **Define access** — native file picker to select folders; access level set per folder
5. **Connect a channel** — optional Telegram/Discord setup (can skip)
6. **First task** — a suggested starter task to confirm everything works

Target: install to first successful agent action in under 5 minutes.

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

## Distribution

Batchismo ships as a single installer per platform:

| Platform | Format |
|---|---|
| macOS | `.dmg` (code-signed and notarized) |
| Windows | `.exe` (NSIS) or `.msi` (code-signed) |
| Linux | `.AppImage`, `.deb`, `.rpm` |

No internet connection required after install except for LLM API calls. Auto-update is built in via Tauri's updater — updates are signed and apply with one click.

---

## Security

- API keys are stored in the OS keychain and never written to disk in plaintext
- Keys are never passed to agent processes — the gateway injects them into each API request on the agent's behalf
- Each agent session runs in an isolated OS process:
  - **Linux:** PID + mount + network namespaces, cgroup v2 limits, seccomp-bpf allowlist
  - **macOS:** Seatbelt sandbox profile generated per-session from path policy
  - **Windows:** Job Objects with memory/CPU limits, restricted token for filesystem access
- Path policy is enforced at both the kernel level and the tool layer
- Full append-only audit log of every agent action — cannot be disabled or deleted
- Memory file writes go through a temp file → gateway validation → atomic rename pipeline, with 30-day rolling history kept in `.history/`

---

## Project Status

Batchismo is in active development.

| Phase | Goal | Status |
|---|---|---|
| 1 | Core agent loop: chat, fs tools, SQLite, streaming | ✅ Complete |
| 2 | Installer, onboarding wizard, all platforms, tray | 🔜 Planned |
| 3 | Memory system: self-updating MEMORY.md, PATTERNS.md | 🔜 Planned |
| 4 | Subagents, OS-native sandboxing, audit log UI | 🔜 Planned |
| 5 | Skill system, web tools, metrics dashboard | 🔜 Planned |

---

## License

Private — Nightfall Advisors. All rights reserved.
