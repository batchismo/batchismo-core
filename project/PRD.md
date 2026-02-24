# Product Requirements Document
# Batchismo
**Version:** 0.1.0  
**Status:** Draft  
**Owner:** Vatché Chamlian / Nightfall Advisors  
**Last Updated:** 2026-02-20

---

## Table of Contents

1. [Vision](#1-vision)
2. [Problem Statement](#2-problem-statement)
3. [Goals and Non-Goals](#3-goals-and-non-goals)
4. [Users](#4-users)
5. [System Architecture](#5-system-architecture)
6. [Core Subsystems](#6-core-subsystems)
7. [Self-Learning and Memory System](#7-self-learning-and-memory-system)
8. [Filesystem Access and Path Policy](#8-filesystem-access-and-path-policy)
9. [Isolation and Security Model](#9-isolation-and-security-model)
10. [User Interface — Desktop (Web)](#10-user-interface)
10b. [User Interface — Terminal (TUI)](#10b-user-interface--terminal-tui)
11. [Onboarding Flow](#11-onboarding-flow)
12. [Distribution and Installation](#12-distribution-and-installation)
13. [Configuration Schema](#13-configuration-schema)
14. [File and Directory Structure](#14-file-and-directory-structure)
15. [Agent MD Files](#15-agent-md-files)
16. [Channel Adapters](#16-channel-adapters)
17. [Tool System](#17-tool-system)
18. [Subagent System](#18-subagent-system)
19. [Audit and Observability](#19-audit-and-observability)
20. [Build and Iteration Strategy](#20-build-and-iteration-strategy)
21. [Success Metrics](#21-success-metrics)
22. [Open Questions](#22-open-questions)

---

## 1. Vision

Batchismo is a locally-installed, OS-native AI agent platform that runs persistently on the user's machine. It acts as a personal AI that does real work — executing tasks, managing files, interacting with services, and coordinating parallel subagents — while continuously learning how each individual user works and updating its own behavior files to reflect that learning over time.

Batchismo ships as a single installable application. No terminal. No Docker. No configuration files to hand-edit. A non-developer can install it, define what folders and services it can touch, and have a working personal AI agent within minutes.

The system is built in Rust for performance, security, and cross-platform native distribution. Agent isolation is achieved using OS-native primitives — namespaces and cgroups on Linux, seatbelt on macOS, Job Objects on Windows — providing container-equivalent security without requiring Docker or any external runtime.

---

## 2. Problem Statement

Existing AI agent platforms (OpenClaw and its successors) require technical users, expose dangerous defaults, and are increasingly controlled by large AI companies with misaligned incentives. They do not learn from the individual user over time. They do not adapt their behavior based on observed patterns. Their isolation models are weak or nonexistent. And they require a developer to install and maintain them.

The gap in the market is a secure, self-improving, non-developer-friendly AI agent runtime that runs entirely on the user's own hardware, respects the user's defined boundaries, and gets meaningfully smarter about that specific user over time.

---

## 3. Goals and Non-Goals

### Goals

- Ship a single-binary desktop application that non-developers can install and use
- Provide a browser-window UI that opens automatically on launch
- Execute real tasks on the user's machine within user-defined path boundaries
- Run subagents concurrently while the user continues conversing with the main agent
- Learn user behavior patterns and update its own MD configuration files to reflect that learning
- Enforce OS-native process isolation per agent session — no Docker required
- Support Windows, macOS, and Linux from day one
- Be model-agnostic with Claude (Anthropic) as the primary and recommended backend
- Provide full audit logging of every agent action, tool call, and file access
- Allow users to define filesystem access policy through a UI — no config file editing

### Non-Goals (v1)

- Cloud sync or multi-device support
- Multi-user / team collaboration features
- Mobile companion apps (deferred to v2)
- A public skill marketplace
- Voice input / output
- Any feature that requires a privileged daemon or root access

---

## 4. Users

### Primary User: The Intelligent Non-Developer

Someone who understands technology and wants AI to do real work for them, but will not and should not need to use a terminal. They may be a knowledge worker, executive, consultant, researcher, or small business owner. They want an AI that learns their habits, manages their files and communications, and works in the background while they focus on other things.

**Characteristics:**
- Comfortable installing software from a website
- Has an Anthropic or OpenAI API key (or is willing to get one)
- Wants AI to take actions, not just answer questions
- Has real privacy and security concerns about what the AI can touch
- Will not tolerate a setup process that involves the command line

### Secondary User: The Technical Power User

A developer or technical professional who wants to extend the system, build custom skills, and inspect what the agent is doing. They may also be using this as infrastructure for their own clients or products.

**Characteristics:**
- Comfortable with JSON/TOML config files and reading logs
- Wants to write custom tools and skills
- May want to run multiple agent profiles for different contexts
- Values the audit trail and observability features

---

## 5. System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Tauri Shell (Rust)                        │
│         Window management · Tray · Auto-start · Auto-update      │
└─────────────────────────────┬───────────────────────────────────┘
                              │ Tauri IPC
┌─────────────────────────────▼───────────────────────────────────┐
│                      Gateway Runtime (Rust)                      │
│                                                                  │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │
│  │   Session   │  │   Channel    │  │     Event Bus          │  │
│  │   Manager   │  │   Router     │  │  (tokio broadcast)     │  │
│  └──────┬──────┘  └──────┬───────┘  └────────────────────────┘  │
│         │                │                                       │
│  ┌──────▼──────────────────────────────────────────────────┐    │
│  │                  Sandbox Orchestrator                    │    │
│  │   Spawns and manages isolated agent processes           │    │
│  └──────┬──────────────────────────────────────────────────┘    │
└─────────│────────────────────────────────────────────────────────┘
          │ OS-native isolation (namespaces / seatbelt / Job Objects)
    ┌─────┴──────┐     ┌────────────────┐     ┌────────────────┐
    │   Agent    │     │   Subagent     │     │   Subagent     │
    │  Process   │     │   Process      │     │   Process      │
    │            │     │                │     │                │
    │ LLM loop   │     │ Isolated task  │     │ Isolated task  │
    │ Tool exec  │     │ Own FS view    │     │ Own FS view    │
    │ Memory R/W │     │ Memory capped  │     │ Memory capped  │
    └────────────┘     └────────────────┘     └────────────────┘
```

All gateway components are compiled into both the Tauri desktop binary (`bat-shell`) and the terminal binary (`bat-tui`). No external daemon. No separate server process the user must manage. Users choose whichever interface they prefer — both embed the same gateway.

---

## 6. Core Subsystems

### 6.1 Gateway Runtime

The central coordinator. Runs as an async Rust process (`tokio` runtime) embedded in the Tauri application. Responsible for:

- Maintaining the session registry (active sessions, their state, their metadata)
- Routing inbound messages from channels to the correct session lane
- Spawning and supervising isolated agent processes
- Exposing a WebSocket control plane on `localhost` for the UI and channel adapters
- Persisting session state to SQLite
- Emitting structured events to the audit log

The gateway does not execute LLM calls or tool calls directly. It delegates those to agent processes.

### 6.2 Session Manager

Each conversation context is a named session with a unique key. Sessions are persistent across app restarts.

**Session types:**
- `main` — the primary direct conversation with the user
- `subagent:<uuid>` — a spawned background task
- `channel:<adapter>:<id>` — a conversation via an external channel (Telegram, Discord, etc.)
- `cron:<job-id>` — a scheduled background task

**Per-session state stored in SQLite:**
- Full conversation transcript
- Active model and thinking level
- Path policy override (inherits from global if not set)
- Token usage
- Last activity timestamp
- Spawn parent (for subagents)

### 6.3 Agent Process

Each active session runs in a separate OS process. The agent process is a compiled Rust binary bundled inside the application. It:

- Receives task context over a Unix socket / named pipe from the gateway
- Runs the LLM loop (tool use via Anthropic API)
- Executes approved tools within its sandbox boundary
- Streams results back to the gateway
- Has read/write access only to paths the user has permitted

The agent process never communicates with the outside world directly except through the LLM API endpoint and tool-specific allowed endpoints.

---

## 7. Self-Learning and Memory System

This is a core differentiating feature of Batchismo. The system observes how the user works and updates its own behavior files over time. This is not a separate AI model — it uses the same LLM backend, triggered on a schedule and on specific events.

### 7.1 Observation Layer

The gateway passively observes patterns across all sessions:

- Which tools the user invokes most frequently
- Which file paths they reference
- What time of day they are most active
- Which tasks they repeatedly ask for
- Which responses they accept, edit, or reject
- Which subagent tasks succeed vs. fail or get cancelled

This data is stored in a structured observation log in SQLite, separate from the conversation transcript. Raw conversation content is never stored in the observation log — only behavioral metadata.

### 7.2 Memory MD Files

The agent's behavior is controlled by a set of Markdown files stored in the user's Batchismo workspace directory. These files are human-readable, human-editable, and also writable by the agent itself.

```
~/.batchismo/workspace/
├── IDENTITY.md       ← who the agent is, its name, its role
├── MEMORY.md         ← persistent facts about the user the agent has learned
├── PATTERNS.md       ← observed behavioral patterns and preferences
├── SKILLS.md         ← index of available skills and when to use them
├── TOOLS.md          ← tool usage preferences and constraints
└── skills/
    ├── email/
    │   └── SKILL.md
    ├── calendar/
    │   └── SKILL.md
    └── files/
        └── SKILL.md
```

**MEMORY.md** is the primary self-updating file. It contains facts the agent has learned about the user:

```markdown
# User Memory

## Work Patterns
- Prefers responses in bullet points for technical topics
- Works primarily between 8am and 11pm EST
- Reviews email in two batches: morning and late afternoon
- Prefers Claude Opus for complex reasoning tasks

## Frequently Accessed Paths
- ~/Documents/Clients/ — primary client work directory
- ~/Projects/ — active development projects
- ~/Desktop/Inbox/ — staging area for files to be processed

## Preferences
- Prefers concise responses unless explicitly asking for detail
- Does not want the agent to send emails without confirmation
- Wants subagents to report back with a summary, not full output
```

**PATTERNS.md** stores higher-level behavioral observations:

```markdown
# Observed Patterns

## Recurring Tasks
- Every Monday: review and triage ~/Documents/Clients/ for new files
- Weekly: summarize project status across ~/Projects/

## Tool Preferences
- Uses file search before asking the agent to find things
- Prefers calendar events created with 15-minute buffer before meetings
```

### 7.3 Memory Update Triggers

The agent updates its memory files under these conditions:

**Automatic triggers:**
- After every 10 completed sessions (lightweight review)
- When a user explicitly corrects the agent ("don't do it that way, do it this way")
- When a recurring pattern is detected (same task requested 3+ times)
- On a configurable daily/weekly schedule (full memory consolidation)

**User-triggered:**
- User can say "remember that..." or "update your memory to reflect..."
- User can open the Memory tab in the UI and edit files directly
- User can tell the agent to forget something

### 7.4 Memory Update Process

When a memory update is triggered, the gateway spawns a dedicated memory-consolidation agent session. This session:

1. Reads the observation log for the relevant period
2. Reads the current MEMORY.md and PATTERNS.md
3. Calls the LLM to identify new facts, corrections, and patterns
4. Produces a diff of proposed changes
5. **In auto-update mode:** applies the diff and logs what changed
6. **In review mode (configurable):** shows the user the proposed changes before applying

The memory consolidation agent has read access to the observation log and read/write access to the workspace MD files. It has no access to conversation transcripts or the broader filesystem.

### 7.5 Memory Boundaries

The system never writes the following to memory files:

- Raw conversation content
- File contents (only file paths and metadata)
- Credentials or API keys
- Anything the user has explicitly told it to forget

---

## 8. Filesystem Access and Path Policy

### 8.1 Design Principle

The agent has **no filesystem access by default**. The user explicitly grants access to specific paths through the UI. This is not a setting buried in a config file — it is a first-class onboarding step and a persistent UI element.

### 8.2 Path Policy Model

Each allowed path has an associated policy:

```toml
[[paths]]
path = "~/Documents/Clients/"
access = "read-write"
recursive = true
description = "Client work files"

[[paths]]
path = "~/Downloads/"
access = "read-only"
recursive = false
description = "Can read files I drop here, cannot modify"

[[paths]]
path = "~/Projects/"
access = "read-write"
recursive = true
description = "Active development projects"
```

**Access levels:**
- `read-only` — agent can read files but not modify or create them
- `read-write` — agent can read, create, and modify files
- `write-only` — agent can deposit files but not read existing content (useful for output folders)

### 8.3 Path Policy Enforcement

Path policy is enforced at two levels:

**Level 1 — Agent process sandbox:** The OS-level sandbox restricts which paths the agent process can even open at the kernel level. If the path isn't in the policy, the `open()` syscall fails before the agent code even runs.

**Level 2 — Tool layer validation:** Before any file tool executes, the tool validator checks the target path against the policy. This provides a second enforcement layer and produces user-readable error messages.

### 8.4 Path Policy UI

The UI provides a dedicated **Access** panel where users can:

- Add new paths via a native OS file picker (they never type paths manually)
- Set access level per path with a dropdown
- See which paths each agent session has accessed in the last 24 hours
- Revoke access to a path with one click
- See a real-time indicator when an agent is actively reading or writing a path

### 8.5 Subagent Path Inheritance

Subagents inherit the path policy of their parent session by default. The spawning agent can further restrict (but not expand) the subagent's path access. A subagent can never have more filesystem access than its parent.

---

## 9. Isolation and Security Model

### 9.1 Per-Agent Process Isolation

Every agent session runs in a separate OS process with the following isolation applied at spawn time:

**Linux:**
- New PID namespace (isolated process tree)
- New mount namespace (isolated filesystem view via bind mounts to allowed paths only)
- New network namespace with only the LLM API endpoint routable
- cgroup v2 memory limit (configurable, default 512MB per agent)
- cgroup v2 CPU shares (configurable, default equal shares)
- seccomp-bpf filter allowing only required syscalls (approximately 40 syscalls)

**macOS:**
- Seatbelt sandbox profile generated per-session from path policy
- `deny default` base — everything denied unless explicitly allowed
- File access rules generated from the user's path policy
- Network restricted to LLM API endpoint only
- `sandbox_init()` applied before exec

**Windows:**
- Job Object with memory and CPU limits
- AppContainer or restricted token for filesystem access
- Windows Filtering Platform rules for network isolation

### 9.2 Syscall Allowlist (Linux)

The default seccomp allowlist for agent processes includes only what is required for LLM API calls, file I/O within allowed paths, and inter-process communication with the gateway:

```
read, write, open, close, stat, fstat, lstat, poll, lseek,
mmap, mprotect, munmap, brk, rt_sigaction, rt_sigprocmask,
ioctl, pread64, pwrite64, readv, writev, access, pipe, select,
sched_yield, mremap, msync, mincore, madvise, dup, dup2,
nanosleep, getpid, sendfile, socket, connect, accept, sendto,
recvfrom, shutdown, setsockopt, getsockopt, clone, fork, execve,
exit, wait4, kill, uname, fcntl, getdents, getcwd, chdir,
rename, mkdir, rmdir, unlink, readlink, chmod, getrlimit,
getuid, getgid, geteuid, getegid, exit_group, futex, clock_gettime
```

Everything else kills the process (`SECCOMP_RET_KILL_PROCESS`).

### 9.3 Network Policy

Agents may only make outbound connections to:

- The configured LLM API endpoint (e.g., `api.anthropic.com:443`)
- Endpoints explicitly listed in the active skill's tool configuration
- The gateway's local Unix socket

Inbound connections from the agent process are never permitted.

### 9.4 Credential Handling

- API keys are stored in the OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Keys are never written to disk in plaintext
- Keys are never passed to agent processes as environment variables — they are injected by the gateway into each API request on behalf of the agent
- The agent process never has direct access to credentials

### 9.5 Memory File Write Safety

When the memory consolidation agent writes to MD files:

- It writes to a temporary file first
- The gateway validates the diff (no binary content, no credential patterns, size limits enforced)
- The gateway performs an atomic rename to replace the file
- The previous version is preserved in a `.batchismo/workspace/.history/` directory (rolling 30-day retention)

---

## 10. User Interface

The UI is a WebView-based interface served from within the Tauri binary. It uses React with a clean, minimal design. It feels like a native app — no visible URL bar, no browser chrome, no localhost address.

### 10.1 Main Panels

**Chat Panel (default view)**
- The primary conversation interface with the main agent session
- Streaming response output with tool call visibility (expandable)
- Typing indicator while agent is working
- Subagent status indicators showing background tasks in progress
- Ability to continue chatting while subagents are running

**Activity Panel**
- Real-time view of all active agent sessions
- Each subagent shown with its task description, elapsed time, and status
- Ability to cancel a running subagent
- View the transcript of any active or completed subagent session

**Memory Panel**
- Displays current MEMORY.md and PATTERNS.md in a readable format
- Edit button opens the file in an in-app editor
- History view showing recent memory updates with diffs
- "What do you know about me?" shortcut that summarizes current memory

**Access Panel**
- Visual list of all permitted paths with their access levels
- Add path button opens native OS file picker
- Per-path activity indicator (last accessed, access count today)
- One-click revoke for any path
- Real-time indicator when a path is being actively read/written

**Skills Panel**
- List of installed skills
- Enable/disable toggle per skill
- Per-skill configuration (each skill defines its own config schema)
- Install new skill from a local folder

**Settings Panel**
- LLM provider selection and API key entry
- Default model selection
- Memory update mode (auto / review / manual)
- Subagent concurrency limit
- Resource limits per agent (memory cap, CPU shares)
- Channel adapter configuration (Telegram, Discord, etc.)
- Auto-start on login toggle

### 10.2 Persistent Status Bar

A thin status bar at the bottom of the window shows:

- Active session count
- Current model
- Token usage for current session
- Gateway health indicator
- A subtle animation when any agent is actively working

### 10.3 Tray Icon

The system tray icon provides:

- Open/close main window
- Quick status (number of active agents)
- Pause all agents
- Quit

The icon pulses subtly when a background agent is active.

---

## 10b. User Interface — Terminal (TUI)

Batchismo ships a **terminal UI (`bat-tui`)** as an alternative frontend for developers and power users. It connects to the same gateway runtime as the desktop app, providing full feature parity through a keyboard-driven interface.

### 10b.1 Architecture

`bat-tui` is a standalone Rust binary (separate crate) that communicates with the gateway library directly — no Tauri dependency. It uses `ratatui` + `crossterm` for rendering.

```
┌──────────────────────────────────────────────┐
│              bat-tui (terminal)               │
│  ratatui · crossterm · tokio                  │
│                                               │
│  Embeds the same bat-gateway library crate    │
│  as bat-shell (Tauri), but renders to the     │
│  terminal instead of a WebView.               │
└──────────────────────────────────────────────┘
```

Both `bat-shell` (desktop) and `bat-tui` (terminal) import `bat-gateway` as a library. The gateway does not care which frontend is driving it.

### 10b.2 Screens

**Chat Screen (default)**
- Full conversation view with scrollable message history
- Streaming response display (tokens appear as they arrive)
- Tool call blocks shown inline (expandable/collapsible with Enter)
- Multi-line input area at the bottom (Shift+Enter for newline, Enter to send)
- Status line showing model, token count, and active session

**Settings Screen**
- Navigable with arrow keys (↑/↓ to move between fields)
- Sub-pages: Agent Config, Path Policies, Tools, About
- Space/Enter to toggle options, edit text fields, or confirm
- API key field with masked display (press Space to reveal/hide)
- Tab to cycle between sub-pages

**Path Policies Screen**
- List view of all policies with path, access level, recursive flag
- Arrow keys to select a policy
- Enter to edit, `a` to add new, `d` to delete (with confirmation)
- Access level cycles through read-only → read-write → write-only with Space

**Activity Screen** (future — subagent support)
- List of active/completed subagent sessions
- Arrow keys to select, Enter to view transcript
- `c` to cancel a running subagent

### 10b.3 Navigation

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle between screens (Chat → Settings → Activity) |
| `↑` / `↓` | Scroll messages (chat) or move between items (settings) |
| `Enter` | Send message (chat) / confirm selection (settings) |
| `Space` | Toggle option / expand tool call block |
| `Esc` | Back to chat / cancel current input |
| `Ctrl+C` | Quit |
| `?` | Show key bindings help overlay |

### 10b.4 Feature Parity with Desktop UI

The TUI must support every feature the desktop web UI supports at each phase. Both interfaces are first-class — the TUI is not a secondary or debug-only tool.

| Feature | Desktop (bat-shell) | Terminal (bat-tui) |
|---|---|---|
| Chat with streaming | ✅ | ✅ |
| Tool call display | ✅ | ✅ |
| Settings management | ✅ | ✅ |
| Path policy CRUD | ✅ | ✅ |
| Status bar | ✅ | ✅ |
| Activity/subagents | Phase 4 | Phase 4 |
| Memory panel | Phase 3 | Phase 3 |

---

## 11. Onboarding Flow

The first-launch experience is a guided wizard. No documentation reading required.

**Step 1: Welcome**
Brief explanation of what Batchismo does. "Your AI that actually works on your computer."

**Step 2: LLM Provider**
Choose provider (Anthropic recommended, OpenAI available). Enter API key. The app validates the key immediately with a test call. Key is stored in OS keychain.

**Step 3: Give It a Name**
The user names their agent. This name is written to `IDENTITY.md`. Establishes the personal relationship.

**Step 4: Define Access**
"What folders can your agent work with?" Native file picker. User selects one or more folders. Sets access level for each. The UI is explicit: "Your agent cannot touch anything else."

**Step 5: Connect a Channel (Optional)**
"Want to talk to your agent from your phone?" Optional Telegram or Discord setup. User can skip and use the built-in chat only.

**Step 6: First Task**
A suggested starter task based on what folders they granted access to. Gets the user to their first successful agent action within 2 minutes of install.

---

## 12. Distribution and Installation

### 12.1 Packaging

Batchismo ships as a single installer per platform:

- **macOS:** `.dmg` with code signing and notarization
- **Windows:** `.exe` NSIS installer or `.msi` with code signing
- **Linux:** `.AppImage` (no install required) and `.deb` / `.rpm`

The installer contains:
- The Tauri shell binary
- The gateway runtime (compiled Rust, embedded in Tauri)
- The agent process binary (bundled as a Tauri resource)
- Default skill definitions (MD files)
- The WebView UI assets

No internet connection required after install except for LLM API calls.

### 12.2 Auto-Update

Tauri's built-in updater checks for updates on launch and in the background. Updates are signed. The update process is silent — the user sees a notification that an update is available and can apply it with one click. No reinstall, no re-onboarding.

### 12.3 Auto-Start

On first launch (after onboarding), the user is asked if they want Batchismo to start automatically when they log in. If yes, Batchismo registers itself as a login item via:

- **macOS:** `launchd` user agent
- **Windows:** Registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- **Linux:** `systemd --user` service or XDG autostart entry

When auto-started, Batchismo launches in tray-only mode. The main window does not open until the user clicks the tray icon or a channel message comes in.

---

## 13. Configuration Schema

Batchismo uses TOML for all configuration files. The primary config lives at `~/.batchismo/config.toml`. Users should rarely need to edit this directly — the UI manages it — but it is human-readable and human-editable for power users.

```toml
[agent]
name = "Aria"                          # set during onboarding
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

---

## 14. File and Directory Structure

```
~/.batchismo/
├── config.toml                    ← main configuration
├── Batchismo.db                         ← SQLite: sessions, transcripts, observations
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

## 15. Agent MD Files

### 15.1 IDENTITY.md

Written during onboarding, edited by user. Read-only to the agent (agent cannot change its own identity without user confirmation).

```markdown
# Agent Identity

## Name
Aria

## Role
Personal AI assistant for Vatché. Focused on client work, research, and 
business development for Nightfall Advisors.

## Personality
Direct, efficient, technically precise. Asks clarifying questions before 
taking irreversible actions. Confirms before sending any external communications.

## Core Constraints
- Never send emails or messages without explicit user confirmation
- Never delete files — move to a trash staging folder instead
- Always explain what you're about to do before doing it for first-time actions
```

### 15.2 MEMORY.md

Maintained by the memory consolidation process. The agent reads this at the start of every session. The user can edit it directly.

See Section 7.2 for example content.

### 15.3 PATTERNS.md

Higher-level behavioral patterns detected over time. Updated less frequently than MEMORY.md (weekly by default).

### 15.4 SKILL.md (per skill)

Each skill defines itself in a structured MD file that the agent reads to understand when and how to use the skill.

```markdown
# Skill: File Management

## Purpose
Read, organize, search, and manage files within permitted paths.

## When to Use
- User asks to find, move, rename, or organize files
- User asks what's in a folder
- User asks to clean up or sort files by type/date

## Tools Available
- `fs.read` — read file contents
- `fs.list` — list directory contents  
- `fs.move` — move file to new path (within allowed paths)
- `fs.search` — search for files by name or content

## Constraints
- Never delete files. Use fs.move to ~/Trash-Staging/ instead.
- Always confirm before moving more than 5 files at once
- Log all file operations to the audit trail

## Notes
User prefers files organized by client name, then by date.
```

---

## 16. Channel Adapters

Channel adapters connect external messaging platforms to the gateway. Each adapter is a module that:

- Authenticates with the external platform
- Receives inbound messages and routes them to the appropriate session
- Delivers outbound messages back to the platform
- Runs as a task within the gateway process (not a separate sandboxed process)

### Supported Channels (v1)

- **Built-in WebChat** — always available, served from the Tauri window
- **Telegram** — via Bot API (long polling or webhook)
- **Discord** — via discord.js-equivalent Rust crate or webhook

### Planned Channels (v2)

- Slack
- WhatsApp (via Baileys or Cloud API)
- iMessage via BlueBubbles

### Channel Security

- Each channel has an `allow_from` list — only listed users/numbers can interact with the agent
- Unknown senders receive no response by default
- DM pairing code flow (same as OpenClaw) for controlled onboarding of new contacts

---

## 17. Tool System

Tools are the actions the agent can take. Every tool is defined with a name, description, input schema, output schema, and a set of preconditions the sandbox must satisfy before the tool runs.

### 17.1 Built-in Tools

**Filesystem tools** (gated by path policy):
- `fs.read` — read file contents
- `fs.write` — write file contents
- `fs.list` — list directory
- `fs.move` — move/rename file
- `fs.search` — search by name or content
- `fs.stat` — get file metadata

**Process tools** (requires explicit user permission):
- `process.run` — execute a shell command in a restricted environment
- `process.run` is disabled by default and requires the user to enable it in Settings

**Web tools**:
- `web.fetch` — fetch a URL (HTTP GET only, no authentication forwarding)
- `web.search` — search the web via configured search API

**Memory tools**:
- `memory.read` — read current MEMORY.md content
- `memory.propose_update` — propose a change to MEMORY.md (triggers review flow if mode is `review`)
- `memory.write_patterns` — write to PATTERNS.md (memory consolidation agent only)

**Session tools**:
- `session.list` — list active sessions
- `session.spawn` — spawn a subagent with a task
- `session.history` — read history of a session
- `session.send` — send a message to another session

### 17.2 Tool Execution Flow

```
Agent requests tool call
        │
        ▼
Tool validator checks:
  - Is this tool enabled for this session?
  - Do the parameters satisfy path policy?
  - Does the sandbox satisfy the tool's preconditions?
        │
   Pass │   Fail ──► Return structured error to agent
        ▼
Tool executes within sandbox
        │
        ▼
Result sanitized (size limits, binary content stripped)
        │
        ▼
Result logged to audit trail
        │
        ▼
Result returned to agent
```

### 17.3 Skill-Defined Tools

Skills can define additional tools by including a `tools.toml` in their skill directory. The gateway registers these tools on skill load. Skill tools run through the same validation and audit pipeline as built-in tools.

---

## 18. Subagent System

### 18.1 Spawning

The main agent can spawn subagents using the `session.spawn` tool. Subagents are always non-blocking — the main agent receives a `{ session_key, run_id, status: "accepted" }` response immediately and continues its work.

```
Main agent calls session.spawn({
  task: "Review all PDF files in ~/Documents/Clients/Acme/ 
         and create a summary document",
  label: "Acme PDF Review",
  memory_limit_mb: 256,
  path_policy: "inherit"   // inherits parent's path access, cannot expand it
})
```

### 18.2 Concurrent Execution

Multiple subagents run concurrently. Each runs in its own isolated process. The UI shows all active subagents in the Activity Panel. The user can continue chatting with the main agent while subagents work.

Maximum concurrent subagents is configurable (default: 5).

### 18.3 Completion and Announce

When a subagent completes, the gateway runs an announce step: the subagent produces a structured summary of what it did and what it found. This summary is posted back to the main session as a system message. The user sees a notification in the UI.

The announce format is standardized:

```
[Subagent: Acme PDF Review — completed in 4m 12s]

Status: Success
Result: Reviewed 14 PDF files. Created summary at 
        ~/Documents/Clients/Acme/Summary-2026-02-19.md
Notes: 3 files were empty. 2 files contained action items 
       flagged in the summary.
```

### 18.4 Subagent Constraints

- Subagents cannot spawn their own subagents (no recursive spawning)
- Subagents cannot access the memory write tools (only the main agent and memory consolidation agent can update MD files)
- Subagents inherit but cannot expand path policy
- Subagents are auto-archived after 60 minutes by default

---

## 19. Audit and Observability

Every agent action is logged to an append-only audit log. This is not optional — it cannot be disabled. Users can view the audit log in the UI but cannot delete entries.

### 19.1 Audit Log Schema

Each audit entry is a structured JSON line:

```json
{
  "ts": "2026-02-19T14:23:11.442Z",
  "session_key": "main",
  "event": "tool_call",
  "tool": "fs.read",
  "params": { "path": "~/Documents/Clients/Acme/contract.pdf" },
  "result": "success",
  "bytes_read": 48291,
  "duration_ms": 12
}
```

```json
{
  "ts": "2026-02-19T14:23:15.001Z",
  "session_key": "main",
  "event": "memory_update",
  "file": "MEMORY.md",
  "lines_added": 3,
  "lines_removed": 1,
  "trigger": "user_correction"
}
```

### 19.2 Audit UI

The Settings panel includes an Audit Log view that shows:

- Filterable list of all agent actions
- Filter by session, tool type, date range
- Expandable detail per entry
- Export to CSV

### 19.3 Metrics

The gateway tracks and exposes:

- Total sessions created (lifetime)
- Active sessions (current)
- Tool calls per hour
- Token usage per day/week/month
- Memory update frequency
- Subagent success/failure rate

These are displayed in a lightweight dashboard in the UI.

---

## 20. Build and Iteration Strategy

### Phase 1 — Core Loop (Weeks 1-4)

**Goal:** Single working agent that can read/write files and call Claude, with both desktop and terminal interfaces.

- Rust gateway with single session, SQLite persistence
- Agent process with LLM loop (Anthropic tool use)
- `fs.read`, `fs.write`, `fs.list` tools
- Basic path policy enforcement (tool-layer only, no OS sandbox yet)
- Tauri shell with WebView UI — chat panel + settings
- Terminal UI (`bat-tui`) with chat + settings at feature parity with desktop
- Windows as primary target (macOS and Linux to follow)

**Exit criteria:** User can chat with the agent and have it read and write files via either the desktop app or the terminal UI.

### Phase 2 — Desktop App (Weeks 5-8)

**Goal:** Real installer that non-developers can use on all platforms.

- Windows and Linux support
- Onboarding wizard (all 6 steps)
- Access panel UI with native file picker
- Tray icon and auto-start
- Code signing and auto-update
- Telegram channel adapter

**Exit criteria:** Non-technical user can download, install, and complete onboarding in under 5 minutes on Windows, macOS, and Linux.

### Phase 3 — Memory System (Weeks 9-12)

**Goal:** Agent that learns and updates its own files.

- Observation log (behavioral metadata only)
- Memory consolidation agent
- MEMORY.md and PATTERNS.md auto-update pipeline
- Memory panel UI with diff view
- Memory history with rollback

**Exit criteria:** After two weeks of use, MEMORY.md contains accurate, useful facts about the user that improve agent behavior.

### Phase 4 — Subagents and Isolation (Weeks 13-16)

**Goal:** True parallel execution with OS-native isolation.

- `session.spawn` tool
- Activity panel UI
- OS sandbox implementation (Linux namespaces + seccomp, macOS seatbelt, Windows Job Objects)
- Per-agent resource limits
- Audit log and audit UI

**Exit criteria:** User can spawn 3 concurrent subagents, continue chatting, and receive structured results when each completes. Each agent process is verifiably sandboxed.

### Phase 5 — Skill System and Polish (Weeks 17-20)

**Goal:** Extensible, production-ready system.

- Full skill system with `SKILL.md` hot reload
- Additional channel adapters (Discord)
- Web tools (`web.fetch`, `web.search`)
- Metrics dashboard
- Performance tuning

---

## 21. Success Metrics

**Installation success rate:** >90% of downloads result in completed onboarding.

**Time to first successful task:** <5 minutes from install to first completed agent action.

**Memory accuracy:** After 2 weeks, >80% of MEMORY.md entries are confirmed accurate by user.

**Isolation integrity:** Zero cross-session data leaks in security testing.

**Subagent reliability:** >95% of spawned subagents complete or fail gracefully (no hung processes).

**User retention:** >60% of users who complete onboarding are still active after 30 days.

---

## 22. Open Questions

1. **Skill marketplace:** Should there be a curated public registry of skills in v2, or keep it local-only to avoid the security risks OpenClaw encountered with its skill repository?

2. **Memory privacy:** Should memory files be encrypted at rest? The OS provides some protection but a determined attacker with local access could read them. Needs a threat model decision.

3. **Model fallback:** Should the system support automatic failover between LLM providers (e.g., Claude primary, GPT-4o fallback)? Adds complexity but improves reliability.

4. **Agent-to-agent messaging:** OpenClaw's `sessions_send` ping-pong pattern is powerful but complex. Should v1 support agent-to-agent messaging or only one-directional subagent spawning?

5. **Process tool safety:** `process.run` is powerful and dangerous. Should it require a separate opt-in confirmation per command rather than a global enable toggle?

6. **Observation log retention:** How long should behavioral observation data be kept? User should be able to purge it. What is the right default retention period?

7. **Offline mode:** Should the system degrade gracefully when the LLM API is unreachable, or hard-fail? A local model fallback (Ollama) would address this but adds significant scope.

---

---

## 23. Unified Key Registry & Multi-Provider Support

### 23.1 Philosophy

API keys are entered **once** at the provider level, not per-feature. A single OpenAI key unlocks chat, TTS, STT, and embeddings. A single Anthropic key unlocks all Claude models. Features auto-enable based on which keys are present — no duplicate entry, no separate "voice key" vs "chat key."

### 23.2 Key → Feature Mapping

| Provider | Key unlocks |
|---|---|
| **Anthropic** | Claude chat models (Sonnet, Opus, Haiku) |
| **OpenAI** | GPT chat models, Whisper STT, OpenAI TTS (6 voices × 3 models) |
| **ElevenLabs** | ElevenLabs TTS (custom/cloned voices, premium quality) |

Future providers (Google, Mistral, Ollama local) follow the same pattern: one key → all that provider's capabilities.

### 23.3 Voice Provider Selection

The Voice settings page shows a **provider picker** that only displays providers with configured keys:
- **OpenAI TTS:** Hardcoded voice list (alloy, echo, fable, nova, onyx, shimmer), 3 models (gpt-4o-mini-tts, tts-1, tts-1-hd)
- **ElevenLabs TTS:** Voice list fetched dynamically via `GET /v1/voices` API — includes custom/cloned voices
- Provider without a key is greyed out with a hint to add the key in Settings → API Keys

### 23.4 Multi-LLM Model Selection

When multiple chat-capable providers have keys configured:
- **Agent Config** shows models from all available providers (Anthropic + OpenAI)
- User picks a **default model** — used for all new sessions
- Per-session model override supported (future)
- Model picker groups by provider with visual separation

Available models by provider:
- **Anthropic:** claude-opus-4-6, claude-sonnet-4-6, claude-haiku-4-5-20251001
- **OpenAI:** gpt-4o, gpt-4o-mini, gpt-4-turbo, o3-mini (when key present)

### 23.5 Multi-LLM Routing (v0.4.0)

Future: intelligent routing between providers based on task type:
- User-defined preferences ("Use Claude for deep reasoning, GPT for quick tasks")
- Automatic failover if one provider is down
- Cost-aware routing (prefer cheaper model when task is simple)
- Requires building an OpenAI chat completions client alongside the existing Anthropic client

### 23.6 Onboarding Integration

- First API key entered during onboarding unlocks all features for that provider
- OpenAI key step (optional) explains: "This key enables voice responses, speech-to-text, and GPT models — all from one key"
- No redundant key entry anywhere in the app
- Keys manageable post-onboarding via Settings → API Keys

---

---

## 24. Enterprise Administration

### 24.1 Overview

Batchismo supports an optional enterprise mode where organizations deploy the agent to employee devices and manage security policy centrally. Each installed instance acts as an **edge agent** that syncs its configuration from an **admin console**. This model preserves the local-first architecture while giving IT and security teams the controls they need.

The enterprise layer is additive. A standalone install with no admin server behaves exactly as it does today. Enterprise features activate only when an organization endpoint is configured.

### 24.2 Architecture

```
┌─────────────────────────────────────────────────┐
│              Admin Console (web)                 │
│                                                  │
│  Org policy editor · Device registry · Audit     │
│  aggregation · Model allowlist · Tool lockdown   │
└──────────────────────┬──────────────────────────┘
                       │ HTTPS (policy sync)
          ┌────────────┼────────────┐
          │            │            │
     ┌────▼────┐  ┌────▼────┐  ┌────▼────┐
     │  Edge   │  │  Edge   │  │  Edge   │
     │ Agent 1 │  │ Agent 2 │  │ Agent 3 │
     │ (laptop)│  │ (desktop│  │ (laptop)│
     └─────────┘  └─────────┘  └─────────┘
```

### 24.3 Policy Model

Enterprise policy is a superset of the local config. Admin-defined policy acts as a **ceiling** on what the local user can do. Users can further restrict their own agent but never override an admin lockdown.

**Policy layers (highest priority first):**
1. **Admin enforced** - cannot be overridden locally (e.g., "shell_run is disabled org-wide")
2. **Admin default** - applied unless the user explicitly changes it (e.g., "default model is Claude Haiku")
3. **User local** - the user's own settings, constrained by layer 1

### 24.4 Manageable Policy Dimensions

| Dimension | Example admin control |
|---|---|
| **Tools** | Disable shell_run, exec_*, or any tool org-wide |
| **Path policies** | Enforce read-only on certain corporate directories, block access to others entirely |
| **Models** | Restrict to an approved model list (cost control, compliance) |
| **API keys** | Push org-provisioned keys so employees don't need their own |
| **Channels** | Disable Telegram adapter, allow only approved channel integrations |
| **Voice** | Disable TTS/STT org-wide or restrict to specific providers |
| **Sandbox limits** | Set minimum memory/CPU isolation thresholds |
| **Audit retention** | Enforce minimum retention period, require audit log forwarding |
| **Memory system** | Disable auto-update, require review mode, restrict what the agent can learn |
| **Skills** | Whitelist approved skills, block unapproved ones from loading |

### 24.5 Policy Sync

Edge agents poll a configured endpoint on a schedule (default: every 15 minutes) and on startup. The sync flow:

1. Agent sends a lightweight registration payload (device ID, agent version, current policy hash)
2. Server responds with the current org policy (or 304 if unchanged)
3. Agent merges the org policy with local config (admin enforced wins, admin defaults apply where user hasn't customized)
4. Agent continues operating with merged config

If the policy server is unreachable, the agent runs with its **last known policy**. It does not fail open (revert to unrestricted) or fail closed (stop working). It holds steady.

### 24.6 Device Registry

The admin console maintains a registry of all enrolled devices:

- Device ID (generated at first enrollment)
- Agent version
- Last sync timestamp
- Last active timestamp
- Policy compliance status (in sync / drifted / offline)
- Assigned policy group (e.g., "Engineering", "Finance", "Executives")

Devices can be grouped for policy targeting. A policy can apply to all devices, a specific group, or an individual device.

### 24.7 Enrollment

Enrollment is initiated from the edge agent using a one-time enrollment token generated by the admin:

1. Admin generates an enrollment token in the console (scoped to a policy group, expiry)
2. User enters the token in Settings or during onboarding
3. Agent registers with the admin server, receives its device ID and initial policy
4. Subsequent syncs use a device certificate or API key issued during enrollment

No MDM or device management agent is required. Enrollment is voluntary from the user's machine. Unenrollment is possible from either side.

### 24.8 Audit Forwarding

In enterprise mode, the local audit log can be configured to forward events to the admin console:

- **Full forwarding** - every audit entry is sent (for regulated industries)
- **Summary forwarding** - aggregated stats only (tool call counts, token usage, session counts)
- **Alerts only** - only policy violations and errors are forwarded

Forwarding is append-only and asynchronous. If the admin server is unreachable, events queue locally and sync when connectivity returns.

### 24.9 Org-Provisioned Keys

Admins can push API keys to edge agents so individual employees don't need their own accounts:

- Keys are delivered via the policy sync channel (encrypted in transit)
- Keys are stored locally using the same mechanism as user keys (config.toml now, OS keychain in future)
- Admin keys take priority over user keys (org pays, org controls)
- Usage is tracked per-device and reported back to the admin console for cost allocation

### 24.10 Privacy Boundaries

Even in enterprise mode, certain boundaries are maintained:

- **Conversation content is never forwarded** to the admin console. Admins see metadata (session count, tool usage, token usage) but not what the employee asked or what the agent said.
- **Memory files are local only.** MEMORY.md and PATTERNS.md stay on the employee's device. Admins cannot read what the agent has learned about a specific user.
- **Audit forwarding is configurable** and the employee can see exactly what is being forwarded in their local Settings panel.

The admin controls *what the agent can do*, not *what the user talks about*. This is an important distinction for adoption and trust.

### 24.11 Implementation Phases

**Phase A (config groundwork):** Add policy layering to the config system. Every config field gets a `source` attribute (user / admin-default / admin-enforced). The UI shows locked fields with an indicator explaining why.

**Phase B (sync protocol):** Implement the policy sync endpoint and client. Device registration and enrollment flow. Local policy merge logic.

**Phase C (admin console):** Web-based admin interface for policy editing, device management, and audit viewing. Could be a separate product/repo.

**Phase D (audit forwarding):** Async event forwarding pipeline with retry and local queuing.

---

*This document is a living specification. The agent running this system is expected to read, understand, and propose updates to this document as the system evolves.*
