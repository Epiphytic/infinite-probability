# Code Style Guidelines

## Rust

### Error Handling

```rust
use anyhow::{Context, Result};
use thiserror::Error;

// Define domain errors
#[derive(Debug, Error)]
pub enum GearError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Git operation failed: {0}")]
    Git(#[from] git2::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// Use Result for fallible operations
pub fn operation() -> Result<Output, GearError> {
    let config = Config::load()
        .context("Failed to load configuration")?;

    let result = perform_action(&config)
        .context("Action failed")?;

    Ok(result)
}
```

### Documentation

```rust
/// Brief description of the function.
///
/// Longer description if needed, explaining behavior,
/// edge cases, and important details.
///
/// # Arguments
///
/// * `param` - Description of the parameter
/// * `config` - Configuration options
///
/// # Returns
///
/// Description of return value and semantics.
///
/// # Errors
///
/// Returns `GearError::Config` if configuration is invalid.
/// Returns `GearError::Network` if connection fails.
///
/// # Examples
///
/// ```
/// let result = function_name("input", &config)?;
/// assert!(result.is_valid());
/// ```
pub fn function_name(param: &str, config: &Config) -> Result<Output> {
    // Implementation
}
```

### Module Organization

```rust
// lib.rs - Public API surface
pub mod config;
pub mod daemon;
pub mod error;

// Re-export common types
pub use config::Config;
pub use error::GearError;

// Prelude for convenience
pub mod prelude {
    pub use crate::config::Config;
    pub use crate::error::{GearError, Result};
}
```

### Naming Conventions

| Item | Convention | Example |
|------|------------|---------|
| Types | PascalCase | `MergeQueue` |
| Functions | snake_case | `process_merge` |
| Constants | SCREAMING_SNAKE | `MAX_RETRIES` |
| Modules | snake_case | `merge_daemon` |
| Traits | PascalCase | `Mergeable` |

### Formatting

Use `rustfmt` with default settings:
```bash
cargo fmt --check  # CI
cargo fmt          # Auto-format
```

### Linting

Use `clippy` with strict settings:
```bash
cargo clippy -- -D warnings -D clippy::all
```

## Markdown

### Frontmatter

```yaml
---
name: document-name
description: "Brief description"
version: "1.0.0"
---
```

### Structure

- Use ATX headers (`#`, `##`, `###`)
- One sentence per line for better diffs
- Use fenced code blocks with language identifiers
- Include examples where helpful

### AISP Conversion

All markdown documentation must have a corresponding `.aisp` file:

```bash
npx aisp-converter --input docs/file.md --output docs/file.aisp
```

## Git Commits

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

| Type | Description |
|------|-------------|
| feat | New feature |
| fix | Bug fix |
| docs | Documentation only |
| style | Formatting, no code change |
| refactor | Code change, no feature/fix |
| perf | Performance improvement |
| test | Adding/fixing tests |
| build | Build system changes |
| ci | CI configuration |
| chore | Maintenance tasks |

### Examples

```
feat(dash): add worktree isolation for parallel agents

Implement git worktree creation and management for each spawned
agent to ensure complete isolation of working directories.

Closes #123
```

```
fix(memo): correct vector similarity threshold

The similarity threshold was being applied inversely,
causing high-similarity matches to be rejected.

Fixes #456
```

## File Organization

```
src/
├── lib.rs           # Library root, public exports
├── main.rs          # Binary entry point
├── config/
│   ├── mod.rs       # Module root
│   ├── loader.rs    # Config loading logic
│   └── schema.rs    # Config types
├── daemon/
│   ├── mod.rs
│   ├── merge.rs
│   └── memory.rs
└── error.rs         # Error types
```

## Testing Style

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Group related tests
    mod config_loading {
        use super::*;

        #[test]
        fn loads_default_config() {
            // Arrange
            let path = test_fixture("default.toml");

            // Act
            let config = Config::load(&path).unwrap();

            // Assert
            assert_eq!(config.log_level, "info");
        }

        #[test]
        fn handles_missing_file() {
            let result = Config::load("nonexistent.toml");
            assert!(result.is_err());
        }
    }
}
```
