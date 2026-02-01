# Infinite Probability

A Claude Code plugin marketplace for enterprise-scale agentic workflows, powered by a unified Rust binary.

## Installation

```bash
cargo install infinite-probability-core
```

## Usage

### Convert Prose to AISP

```bash
# From file
infinite-probability convert --input document.md --output document.aisp

# From stdin
echo "All users must authenticate before accessing resources" | infinite-probability convert

# With options
infinite-probability convert --input doc.md --output doc.aisp --tier full --verbose
```

### Convert AISP to Prose

```bash
infinite-probability to-prose --input document.aisp --output document.md
```

### Validate AISP Document

```bash
infinite-probability validate --input document.aisp
infinite-probability validate --input document.aisp --json
```

### Detect Conversion Tier

```bash
infinite-probability triage --input document.md
```

### Configuration

```bash
infinite-probability config show                    # Show all config
infinite-probability config get aisp.default_tier   # Get specific value
infinite-probability config path --global           # Show global config path
```

## Configuration Hierarchy

1. Default values
2. Global config (`~/.config/infinite-probability/config.toml`)
3. Project config (`.infinite-probability/config.toml`)
4. Environment variables (`INFINITE_PROBABILITY_*`)
5. CLI flags

## License

MIT
