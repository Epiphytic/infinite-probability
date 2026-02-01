# Configuration System

## Hierarchy (lowest to highest priority)

1. **Default** - Built-in defaults in gear-core
2. **Global** - `~/.config/gear/config.toml`
3. **Local** - `~/.config/gear/config.local.toml` (gitignored)
4. **Project** - `.gear/config.toml`
5. **Project Local** - `.gear/config.local.toml` (gitignored)
6. **Environment** - `GEAR_*` environment variables
7. **CLI flags** - Runtime overrides

## Configuration File Format

```toml
# .gear/config.toml

[general]
log_level = "info"
telemetry = false

[dash]
auto_branch = true
commit_style = "angular"
pr_auto_create = true

[memo]
memory_tier = "project"  # project | workspace | global
vector_dimensions = 1536
similarity_threshold = 0.8

[uplink]
registry_url = "https://uplink.epiphytic.dev"
heartbeat_interval = 30

[guide]
validation_model = "claude-3-5-sonnet"
confidence_threshold = 0.85

[performance]
benchmark_on_pr = true
regression_threshold = 0.05  # 5% regression fails
```

## Plugin-Specific Settings

Store in `.claude/<plugin-name>.local.md`:
```yaml
---
api_key: "secret"
enabled_features:
  - feature1
---
Configuration documentation here
```

## Environment Variables

All gear settings can be overridden via environment:
- `GEAR_LOG_LEVEL` - Logging level
- `GEAR_DASH_AUTO_BRANCH` - Auto-create branches
- `GEAR_MEMO_TIER` - Memory tier
- `GEAR_UPLINK_URL` - Uplink registry URL

## CLI Configuration Commands

```bash
# Read configuration
gear-core cli config get dash.auto_branch
gear-core cli config get --all

# Set configuration (project level)
gear-core cli config set dash.commit_style angular

# Set configuration (global level)
gear-core cli config set --global log_level debug
```

## Secret Management

- Never store secrets in tracked config files
- Use `.local.toml` files (gitignored) for sensitive values
- Prefer environment variables for CI/CD contexts
- Use `gear-core cli secrets` for encrypted storage
