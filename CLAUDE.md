# Infinite Probability Plugin Marketplace

Infinite Probability is a Claude Code plugin marketplace for enterprise-scale agentic workflows, powered by a unified Rust binary (`infinite-probability-core`).

## Essential Rules

1. **Rust-First**: All core logic in Rust, avoid bash scripts for heavy lifting
2. **Performance**: Every PR must pass benchmarks, max 5% regression allowed
3. **Security**: Validate all input, never store secrets in code
4. **Dual Documentation**: All docs in `.md` AND `.aisp` format
5. **Testing**: Unit, integration, and performance tests required on all PRs

## Quick Reference

| Need | Document |
|------|----------|
| System architecture | `docs/architecture.aisp` |
| Building plugins | `docs/plugin-development.aisp` |
| Configuration | `docs/configuration.aisp` |
| Writing tests | `docs/testing.aisp` |
| Security practices | `docs/security.aisp` |
| Performance tuning | `docs/performance.aisp` |
| Code style | `docs/code-style.aisp` |
| Agent development | `AGENTS.aisp` |

## Plugin Suite

| Plugin | Purpose |
|--------|---------|
| infinite-probability | Entry point, project setup (`/first`, `/config`) |
| dash | Git workflow orchestration with isolated worktrees |
| babelfish | Prompt translation to AISP format |
| memo | Distributed vector memory across projects |
| uplink | Cross-machine instance coordination |
| guide | External LLM validation for plans/code |
| tango | Instance spawning (local and remote) |
| infinite-improbability-drive | Meta-orchestration for impossible tasks |

## Core Commands

```bash
# AISP conversion (prose ↔ AISP)
infinite-probability convert --input file.md --output file.aisp     # Prose → AISP
infinite-probability to-prose --input file.aisp --output file.md    # AISP → Prose
infinite-probability validate --input file.aisp                      # Validate AISP
infinite-probability triage --input file.md                          # Detect tier

# Configuration
infinite-probability config get <key>           # Read config value
infinite-probability config show                # Show all config
infinite-probability config path                # Show config path
```

## Directory Structure

```
infinite-probability/
├── CLAUDE.md, CLAUDE.aisp       # Essential instructions
├── AGENTS.md, AGENTS.aisp       # Agent development
├── docs/                        # All docs (.md + .aisp)
├── core/                        # Rust binary source
├── plugins/                     # Individual plugins
├── tests/                       # Test suites
└── .github/workflows/           # CI/CD
```

## Commit Convention

```
<type>(<scope>): <description>
```
Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`

## Configuration Hierarchy

1. Default → 2. Global (`~/.config/infinite-probability/`) → 3. Project (`.infinite-probability/`) → 4. Environment (`GEAR_*`) → 5. CLI flags

Secrets go in `.local.toml` files (gitignored).

## Creating .aisp Files

All documentation must exist in dual format. Convert using:
```bash
infinite-probability convert --input docs/file.md --output docs/file.aisp
```

## Resources

- Human docs: `docs/*.md`
- Machine docs: `docs/*.aisp`
- Agent guide: `AGENTS.md` / `AGENTS.aisp`
