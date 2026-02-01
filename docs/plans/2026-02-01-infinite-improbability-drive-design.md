# infinite-improbability-drive Plugin Design

**Date:** 2026-02-01
**Status:** Approved
**Repository:** https://github.com/epiphytic/infinite-improbability-drive

## Overview

The `infinite-improbability-drive` plugin provides a `spawn` skill for launching sandboxed coding LLMs with intelligent resource provisioning and lifecycle management. It enables the host LLM to delegate complex tasks to isolated sub-LLMs without context pollution.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Host LLM Session                        │
│  (receives spawn request, gets summary results)             │
└─────────────────────┬───────────────────────────────────────┘
                      │ spawn command
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                   LLM Watcher Agent                         │
│  - Evaluates task requirements (LLM-assisted)               │
│  - Provisions sandbox (git worktree)                        │
│  - Monitors progress (files, commits, output)               │
│  - Detects errors & manages recovery                        │
│  - Creates PR & summary on completion                       │
└─────────────────────┬───────────────────────────────────────┘
                      │ provisions & monitors
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                  Sandboxed LLM Instance                     │
│  - Runs in isolated git worktree                            │
│  - Headless, streaming mode, no session history             │
│  - Limited permissions (no --dangerously-skip-permissions)  │
│  - Output piped to debug files                              │
└─────────────────────────────────────────────────────────────┘
```

### Key Components

| Component | Responsibility |
|-----------|----------------|
| `SpawnCommand` | CLI/skill entry point |
| `WatcherAgent` | Orchestrates spawn lifecycle |
| `SandboxProvider` trait | Abstracts isolation (worktree now, Docker later) |
| `LLMRunner` | Launches and streams from target CLI |
| `ProgressMonitor` | Tracks activity, detects hangs |
| `PermissionDetector` | Pattern-matches permission errors |
| `PRManager` | Creates PRs, handles merge conflicts |

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Sandbox isolation | Git worktree (Docker/Podman future) | Lightweight, leverages existing git infrastructure |
| Task evaluation | LLM-assisted with CLI knowledge | Accurate resource provisioning |
| Recovery strategy | Configurable: moderate (default), aggressive, interactive | Flexibility for different use cases |
| Mode selection | Config default + `--aisp` / `--passthrough` flags | Follows config hierarchy pattern |
| Spawn-team coordination | Sequential minimal (default), ping-pong (option) | Clean separation, thoroughness when needed |
| Timeout strategy | Activity-based: idle (2min) + total (30min) | Won't kill thinking LLMs, catches hangs |
| Change integration | PR created, host LLM decides merge | Host retains control |
| Logging | Tiered: standard on success, full on failure | Disk efficiency with debug capability |
| Merge conflicts | Auto-fix small, spawn repair sandbox for large | Robust handling |
| Secrets | CLI injects env vars, redacts from all logs | Security |

## Configuration

```toml
# .infinite-probability/improbability-drive.toml

[spawn]
# Mode: use --aisp or --passthrough CLI flags to override
mode = "aisp"

# Recovery strategy: "moderate", "aggressive", "interactive"
recovery_strategy = "moderate"

# Timeouts in seconds
idle_timeout = 120      # 2 minutes no activity
total_timeout = 1800    # 30 minutes wall clock

# Default LLM for spawned instances
default_llm = "claude-code"  # or "gemini-cli"

# Logging
log_level = "standard"  # Success: standard, Failure: auto-escalates to full

[spawn.permissions]
# Default permission set for sandboxed LLMs
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
denied_tools = ["Task"]  # No recursive spawning by default
max_permission_escalations = 1  # For moderate mode

[spawn-team]
# Coordination mode: "sequential" or "ping-pong"
coordination = "sequential"

# Max iterations for ping-pong mode
max_iterations = 3

# Reviewer LLM (used after primary completes)
reviewer_llm = "gemini-cli"
```

### CLI Options

```bash
# Basic spawn
infinite-improbability-drive spawn "fix the auth bug"

# Mode override
infinite-improbability-drive spawn --aisp "implement feature X"
infinite-improbability-drive spawn --passthrough "simple fix"

# Permission escalation override
infinite-improbability-drive spawn --max-permission-escalations=3 "complex task"

# Timeout override
infinite-improbability-drive spawn --idle-timeout 300 --total-timeout 3600 "big refactor"

# Spawn-team with ping-pong
infinite-improbability-drive spawn-team --coordination ping-pong "implement feature X"
```

## Sandbox Manifest

The watcher agent produces a `SandboxManifest` via LLM-assisted evaluation:

```rust
struct SandboxManifest {
    // Filesystem scope (relative to worktree root)
    readable_paths: Vec<PathPattern>,   // e.g., ["src/**", "tests/**"]
    writable_paths: Vec<PathPattern>,   // e.g., ["src/auth/**"]

    // Tools the sandboxed LLM can use
    allowed_tools: Vec<String>,         // ["Read", "Write", "Edit", "Bash"]

    // Commands the LLM might need to run
    allowed_commands: Vec<CommandPattern>,  // ["npm test", "cargo build"]

    // Environment variables to inject
    environment: HashMap<String, String>,   // {"NODE_ENV": "test"}

    // Secrets (fetched from secure storage, never logged)
    secrets: Vec<SecretRef>,            // ["API_KEY", "DB_PASSWORD"]

    // Estimated complexity for timeout tuning
    complexity: TaskComplexity,         // Low, Medium, High
}
```

### Sandbox Enforcement

- Worktree is created in isolated directory
- `$HOME`, config files, other repos are inaccessible
- No read permissions to files outside the worktree
- Bash commands run with restricted `$PATH`
- Network access follows allowed command patterns only
- Secrets injected as env vars by CLI, stripped from all logs
- `--dangerously-skip-permissions` is never allowed

## LLM Runner

```rust
struct LLMSpawnConfig {
    cli: LLMCli,                    // ClaudeCode, GeminiCli
    prompt: String,                 // Original or AISP-converted
    worktree_path: PathBuf,         // Isolated working directory
    manifest: SandboxManifest,      // Permissions from watcher

    // Speed optimizations
    session_history: false,         // --no-session-persistence
    headless: true,                 // No interactive prompts
    streaming: true,                // Real-time output capture
}

enum LLMCli {
    ClaudeCode {
        allowed_tools: Vec<String>,
        model: String,              // sonnet, haiku, opus
    },
    GeminiCli {
        sandbox_mode: String,
        model: String,
    },
}
```

## Progress Monitoring

```rust
struct ProgressState {
    files_read: HashSet<PathBuf>,
    files_written: HashSet<PathBuf>,
    commits_made: Vec<CommitInfo>,
    output_lines: usize,
    last_activity: Instant,         // For idle timeout
    start_time: Instant,            // For total timeout

    // Error detection
    permission_errors: Vec<PermissionError>,
    other_errors: Vec<String>,
}
```

### Output Capture (Tiered by Outcome)

```
.improbability-drive/spawns/<spawn-id>/
├── prompt.txt              # Original prompt sent
├── prompt.aisp             # AISP version (if converted)
├── manifest.json           # SandboxManifest used
├── stdout.log              # Streaming output (standard)
├── stderr.log              # Error output
├── events.jsonl            # Structured events (tool calls, edits)
├── progress.json           # Final ProgressState
└── debug/                  # Only populated on failure
    ├── raw-stream.log      # Token-by-token capture
    └── full-trace.jsonl    # Complete execution trace
```

## Permission Error Detection & Recovery

### Known Error Patterns

```rust
enum PermissionErrorType {
    FileReadDenied,      // "Permission denied: /path/to/file"
    FileWriteDenied,     // "Cannot write to: /path"
    CommandBlocked,      // "Command not allowed: npm install"
    ToolDisabled,        // "Tool 'Bash' is not enabled"
    EnvVarMissing,       // "Environment variable X not set"
    SecretMissing,       // "API key required but not provided"
    NetworkBlocked,      // "Network access denied"
}

enum PermissionFix {
    AddReadPath(PathPattern),
    AddWritePath(PathPattern),
    AllowCommand(String),
    EnableTool(String),
    InjectEnvVar(String),
    InjectSecret(SecretRef),
    CannotFix(String),   // Requires respawn or fail
}
```

### Recovery Flow

```
Permission error detected
        │
        ▼
┌─────────────────────────┐
│ Analyze error type      │
│ Match against patterns  │
└───────────┬─────────────┘
            │
            ▼
    Can we fix it?
     /          \
   Yes           No
    │             │
    ▼             ▼
Escalation    Kill sandbox
count < max?  Report to host
 /      \
Yes      No
 │        │
 ▼        ▼
Apply fix  Kill sandbox
& retry   Report to host
```

- **Moderate mode (default):** Follows flow above with escalation limit
- **Aggressive mode:** Skips escalation count, keeps trying until `CannotFix`
- **Interactive mode:** Pauses and asks user at decision points

## Spawn-Team Coordination

### Sequential Mode (Default)

```
┌──────────────────────────────────────────────────────────────┐
│                      Watcher Agent                           │
└──────────────────────────┬───────────────────────────────────┘
                           │
     Phase 1: Primary      │      Phase 2: Review
                           │
    ┌──────────────┐       │       ┌──────────────┐
    │ Claude-Code  │       │       │  Gemini-CLI  │
    │              │───────┼──────▶│              │
    │ (AISP/pass)  │  diff │       │ (English)    │
    └──────────────┘       │       └──────────────┘
           │               │              │
           ▼               │              ▼
    Changes in worktree    │       Suggestions JSON
                           │              │
                           │              ▼
                           │       ┌──────────────┐
     Phase 3: Execute      │       │ Claude-Code  │
                           │       │              │
                           │       │ Apply fixes  │
                           │       └──────────────┘
```

**Gemini receives:**
- Original English prompt (never AISP - avoids translation errors)
- Git diff of Claude's changes
- No intermediate reasoning (unbiased review)

**Gemini outputs:**
```json
{
  "verdict": "needs_changes",
  "suggestions": [
    {
      "file": "src/auth.rs",
      "line": 42,
      "issue": "Missing error handling for expired tokens",
      "suggestion": "Add match arm for TokenExpired variant"
    }
  ]
}
```

### Ping-Pong Mode (Optional)

- Same flow but iterates: Claude → Gemini → Claude → Gemini...
- Stops when Gemini returns `"verdict": "approved"` or max iterations hit
- Each iteration sees only the delta from previous round

## PR Creation & Host Summary

### SpawnResult Structure

```rust
struct SpawnResult {
    status: SpawnStatus,            // Success, Failed, TimedOut
    spawn_id: Uuid,
    duration: Duration,

    // Changes made
    files_changed: Vec<FileChange>,
    commits: Vec<CommitInfo>,

    // For host LLM consumption
    summary: String,                // Human-readable summary
    pr_url: Option<String>,         // PR created from worktree

    // For debugging (paths to log files)
    logs: SpawnLogs,
}
```

### PR Creation Flow

1. Watcher commits any uncommitted changes in worktree
2. Pushes worktree branch to origin (or creates local branch if no remote)
3. Creates PR targeting the original branch
4. PR description includes: original prompt, summary, files modified, link to logs

### Merge Conflict Handling

- **Small conflicts** (few lines, clear resolution): Watcher fixes directly
- **Large conflicts** (multiple files, ambiguous): Spawns repair sandbox with conflict context

### Summary for Host LLM

```markdown
## Spawn Complete: fix-auth-bug (spawn-id: abc123)

**Status:** Success (3m 42s)
**PR:** #147 - https://github.com/org/repo/pull/147

### Changes
- `src/auth/token.rs` (+45, -12) - Added token expiry validation
- `src/auth/middleware.rs` (+8, -2) - Updated error handling
- `tests/auth_test.rs` (+67, -0) - Added expiry test cases

### Summary
Implemented token expiry checking in the auth middleware. Added
TokenExpired error variant and corresponding test coverage.

### Logs
Debug logs: `.improbability-drive/spawns/abc123/`
```

## Plugin Structure

```
infinite-improbability-drive/
├── .claude-plugin/
│   └── plugin.json              # Plugin metadata
├── CLAUDE.md                    # Plugin instructions
├── CLAUDE.aisp                  # AISP version
├── README.md                    # User documentation
│
├── commands/
│   ├── spawn.md                 # /spawn command
│   └── spawn-team.md            # /spawn-team command
│
├── skills/
│   └── spawn/
│       └── SKILL.md             # spawn skill definition
│
├── agents/
│   └── watcher.md               # LLM watcher agent definition
│
├── hooks/
│   └── hooks.json               # Event hooks (if needed)
│
├── core/                        # Rust implementation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── main.rs              # CLI binary
│       ├── spawn.rs             # Spawn logic
│       ├── watcher.rs           # Watcher agent orchestration
│       ├── sandbox/
│       │   ├── mod.rs
│       │   ├── provider.rs      # SandboxProvider trait
│       │   ├── worktree.rs      # Git worktree implementation
│       │   └── docker.rs        # Future: container implementation
│       ├── runner/
│       │   ├── mod.rs
│       │   ├── claude.rs        # Claude-code launcher
│       │   └── gemini.rs        # Gemini-cli launcher
│       ├── monitor.rs           # Progress & timeout tracking
│       ├── permissions.rs       # Error detection & recovery
│       ├── secrets.rs           # Secret injection & redaction
│       └── pr.rs                # PR creation & conflict handling
│
├── docs/
│   ├── architecture.md
│   ├── architecture.aisp
│   ├── configuration.md
│   └── configuration.aisp
│
└── tests/
    ├── spawn_test.rs
    ├── watcher_test.rs
    └── integration/
```

## Prerequisites

Update `infinite-probability-core` to use `rosetta-aisp-llm v0.3.0`:

```toml
# core/Cargo.toml
rosetta-aisp-llm = "0.3"  # was 0.1
```

This provides the new CLI options: `--llm-fallback`, `--threshold`, `--model`, `--aisp-prompt`, `--format json`, `round-trip` command.

## Implementation Phases

### Phase 1: Foundation
- Update rosetta-aisp-llm to v0.3.0
- Create plugin structure in infinite-improbability-drive repo
- Implement SandboxProvider trait + worktree implementation
- Basic spawn command (no watcher, manual permissions)

### Phase 2: Watcher Agent
- Implement watcher agent with LLM-assisted evaluation
- Progress monitoring and timeout detection
- Permission error detection with pattern matching

### Phase 3: Recovery & Integration
- Implement recovery strategies (moderate, aggressive, interactive)
- PR creation and merge conflict handling
- Secret injection and log redaction

### Phase 4: Spawn-Team
- Sequential coordination mode
- Ping-pong coordination mode
- Gemini-cli runner integration

### Phase 5: Polish
- Documentation (md + aisp)
- Tests (unit, integration, performance)
- Configuration validation
