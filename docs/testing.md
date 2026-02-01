# Testing Requirements

## Every PR Must Include

1. **Unit Tests** - Core functionality
2. **Integration Tests** - Cross-component interaction
3. **Performance Benchmarks** - Speed regression detection

## Test Structure

```
tests/
├── unit/
│   └── <module>_test.rs
├── integration/
│   └── <feature>_test.rs
└── performance/
    └── benchmarks.rs
```

## Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_load() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_op().await;
        assert!(result.is_ok());
    }
}
```

## Integration Testing

```rust
// tests/integration/workflow_test.rs
use gear_core::prelude::*;

#[tokio::test]
async fn test_full_workflow() {
    let ctx = TestContext::new().await;

    // Setup
    ctx.init_repo().await;

    // Execute
    let result = ctx.run_workflow().await;

    // Verify
    assert!(result.pr_created);
    assert_eq!(result.commits.len(), 1);
}
```

## Performance Benchmarks

```rust
// tests/performance/benchmarks.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn merge_queue_throughput(c: &mut Criterion) {
    c.bench_function("merge 100 agents", |b| {
        b.iter(|| {
            // Benchmark code
        });
    });
}

fn config_read_latency(c: &mut Criterion) {
    c.bench_function("config read", |b| {
        b.iter(|| {
            Config::load().unwrap();
        });
    });
}

criterion_group!(benches, merge_queue_throughput, config_read_latency);
criterion_main!(benches);
```

## CI Configuration

```yaml
# .github/workflows/ci.yml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --all-features

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Check formatting
        run: cargo fmt --check

  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench -- --save-baseline pr

      - name: Compare performance
        run: |
          # Fail if regression > 5%
          cargo bench -- --baseline main --threshold 0.05
```

## Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Build release
        run: cargo build --release

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: target/release/gear-core
          generate_release_notes: true
```

## Test Coverage Requirements

- Minimum 80% line coverage for new code
- Critical paths require 95% coverage
- All public APIs must have tests
