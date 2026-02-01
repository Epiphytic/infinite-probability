# Gear Architecture

## Overview

Gear is a Claude Code plugin marketplace for enterprise-scale agentic workflows. It provides a unified ecosystem of interconnected plugins powered by a common Rust binary (`gear-core`).

## Core Principles

1. **Rust-First**: All heavy lifting encoded in Rust, avoiding bash scripts and AI prompts for core logic
2. **Performance Paramount**: Every PR must pass performance benchmarks
3. **Security by Design**: All plugins built with security and safety as primary concerns
4. **Modular Shared Libraries**: Maximum code reuse through the `gear-core` binary
5. **Unified Configuration**: Global, local, and project-level settings via common system

## Plugin Suite

### gear (Entry Point)
Onboarding plugin that bootstraps new projects and manages the plugin ecosystem.
- `/first` skill: Interactive project setup wizard
- `/config` command: Interface to unified configuration system

### dash (Workflow Orchestration)
Multi-agent git workflow orchestrator with isolated worktrees and FIFO merge queue.
- Isolated git worktrees per agent
- Angular-style commit conventions
- Automatic PR creation and labeling

### babelfish (Prompt Translation)
Transforms natural language prompts into precise AI-optimized instructions using AISP.
- Structured prompt enhancement
- Context injection and ambiguity resolution

### memo (Distributed Memory)
Multi-tiered memory system for cross-project learning.
- Vector-based semantic memory
- Project, workspace, and global tiers

### uplink (Global Orchestration)
Links Claude Code instances for distributed coordination.
- Cross-machine instance registration
- Task distribution and load balancing

### guide (Validation Coordinator)
External LLM validation for plans and code merges.
- Plan review before implementation
- Confidence scoring and risk assessment

### tango (Instance Launcher)
Spawns and manages Claude instances locally and remotely.
- Local and remote instance management
- Resource allocation and pooling

### infinite-improbability-drive (Meta-Orchestration)
Intelligent orchestration layer coordinating all plugins for impossible tasks.
- Multi-machine task decomposition
- Failure recovery

## Unified Rust Binary: gear-core

All plugins share a single Rust binary with multiple modes:

```
gear-core
├── daemon     # Background services (merge queue, memory, uplink)
├── cli        # Command-line interface for all operations
├── ipc        # Inter-process communication (Unix sockets)
├── config     # Configuration management
├── memory     # Vector storage and retrieval
├── git        # Git operations (worktree, merge, branch)
├── translate  # Prompt translation engine
├── network    # Cross-machine communication
└── metrics    # Performance monitoring
```

Invocation:
```bash
gear-core daemon --mode merge-queue
gear-core cli config get <key>
gear-core cli translate "<prompt>"
```

## Directory Structure

```
gear/
├── CLAUDE.md                    # Essential instructions
├── AGENTS.md                    # Agent development
├── .claude-plugin/
│   └── marketplace.json
├── docs/                        # Documentation (.md and .aisp)
│   ├── architecture.md/.aisp
│   ├── plugin-development.md/.aisp
│   ├── configuration.md/.aisp
│   ├── testing.md/.aisp
│   ├── security.md/.aisp
│   └── performance.md/.aisp
├── core/                        # Shared Rust binary
│   ├── Cargo.toml
│   └── src/
├── plugins/
│   ├── gear/
│   ├── dash/
│   ├── babelfish/
│   ├── memo/
│   ├── uplink/
│   ├── guide/
│   ├── tango/
│   └── infinite-improbability-drive/
├── tests/
│   ├── integration/
│   ├── performance/
│   └── e2e/
└── .github/workflows/
```

## Cross-Plugin Communication

Plugins communicate through:
1. **gear-core IPC** - Unix domain sockets
2. **Shared state** - `.gear/` directory
3. **Config system** - Shared configuration
