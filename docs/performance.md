# Performance Guidelines

## Optimization Profile

```toml
# Cargo.toml release profile
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## Latency Thresholds

| Operation | Max Latency |
|-----------|-------------|
| Hook execution | 10ms |
| Config read | 1ms |
| Prompt translation | 50ms |
| Memory query | 20ms |
| Git operation | 100ms |
| IPC message | 5ms |

## Async Best Practices

```rust
use tokio::time::{timeout, Duration};

// Use tokio for async operations
#[tokio::main]
async fn main() -> Result<()> {
    // Spawn concurrent tasks
    let (result1, result2) = tokio::join!(
        async_operation1(),
        async_operation2(),
    );
    Ok(())
}

// Always set timeouts
async fn with_timeout<T>(future: impl Future<Output = T>) -> Result<T> {
    timeout(Duration::from_secs(30), future)
        .await
        .map_err(|_| Error::Timeout)
}
```

## Memory Efficiency

```rust
// Use Arc for shared state
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SharedState {
    inner: Arc<RwLock<StateInner>>,
}

// Stream large data instead of loading fully
pub async fn process_large_file(path: &Path) -> Result<()> {
    let file = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(file);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        process_line(&line)?;
    }

    Ok(())
}

// Use memory-mapped files for persistence
use memmap2::MmapMut;

pub fn mmap_state(path: &Path) -> Result<MmapMut> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    unsafe { MmapMut::map_mut(&file) }
        .map_err(|e| Error::Mmap(e))
}
```

## Caching Strategy

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Cache<K, V> {
    data: HashMap<K, (V, Instant)>,
    ttl: Duration,
}

impl<K: Eq + Hash, V: Clone> Cache<K, V> {
    pub fn get(&self, key: &K) -> Option<V> {
        self.data.get(key).and_then(|(v, t)| {
            if t.elapsed() < self.ttl {
                Some(v.clone())
            } else {
                None
            }
        })
    }
}
```

## Database Optimization

```rust
// Use connection pooling
use sqlx::sqlite::SqlitePoolOptions;

pub async fn create_pool(url: &str) -> Result<SqlitePool> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .map_err(|e| Error::Database(e))
}

// Use prepared statements
pub async fn query_prepared(pool: &SqlitePool, id: i64) -> Result<Record> {
    sqlx::query_as!(Record, "SELECT * FROM records WHERE id = ?", id)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Query(e))
}
```

## Profiling

```rust
// Use tracing for performance monitoring
use tracing::{instrument, info_span};

#[instrument(skip(large_data))]
pub async fn process(large_data: &[u8]) -> Result<Output> {
    let span = info_span!("processing", size = large_data.len());
    let _guard = span.enter();

    // Process...
}

// Enable flamegraph profiling
// cargo install flamegraph
// cargo flamegraph --bin gear-core
```

## Benchmark Requirements

Every PR must:
1. Not regress performance by more than 5%
2. Include benchmarks for new critical paths
3. Document any intentional performance tradeoffs

```rust
// Benchmark template
use criterion::{black_box, criterion_group, Criterion};

fn benchmark_operation(c: &mut Criterion) {
    let setup_data = prepare_data();

    c.bench_function("operation_name", |b| {
        b.iter(|| {
            operation(black_box(&setup_data))
        });
    });
}
```

## Resource Limits

Configure limits to prevent runaway resource usage:

```toml
# .gear/config.toml
[limits]
max_memory_mb = 512
max_open_files = 100
max_concurrent_agents = 8
queue_depth_limit = 100
session_timeout_secs = 3600
```
