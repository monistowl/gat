# CLI Migration Guide

## Flag Changes in v0.6

### Renamed Flags (with backwards compatibility)

| Old Flag | New Flag | Alias |
|----------|----------|-------|
| `--output` | `--out` | `--output` still works |
| `--output-dir` | `--out-dir` | `--output-dir` still works |

### New Short Aliases

| Short | Long | Description |
|-------|------|-------------|
| `-o` | `--out` | Output file path |
| `-d` | `--out-dir` | Output directory |
| `-f` | `--format` | Output format |
| `-t` | `--threads` | Thread count |
| `-m` | `--method` | OPF method |

### New Enum-Based Flags

These flags now use typed enums with tab completion:

- `--format`: `table`, `json`, `jsonl`, `csv`
- `--method`: `economic`, `dc`, `socp`, `ac`
- `--mode`: `dc`, `ac`
- `--rating-type`: `rate-a`, `rate-b`, `rate-c`
