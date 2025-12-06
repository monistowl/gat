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

### New Global Flags

| Flag | Description | Default |
|------|-------------|---------|
| `--base-mva` | System base MVA for per-unit calculations | `100.0` |

### New Command-Specific Flags

| Command | Flag | Description |
|---------|------|-------------|
| `pf dc`, `pf ac` | `--slack-bus <BUS_ID>` | Override automatic slack bus selection |
| `pf dc` | `--stdout-format` | Output format when writing to stdout (`-o -`) |
| `pf ac`, `opf ac`, `opf ac-nlp`, `opf run` | `--show-iterations` | Display per-iteration convergence progress |
| `n-1 dc`, `analytics ds`, `analytics reliability` | `--rating-type` | Which thermal rating to use for limit checks |

### Stdout/Piping Support

Commands now support Unix-style piping with `-o -`:

```bash
# Write JSON to stdout
gat pf dc grid.arrow -o - | jq '.[] | select(.flow_mw > 50)'

# Stream JSON Lines for processing
gat inspect generators grid.arrow --format jsonl | head -5

# Export to CSV
gat inspect branches grid.arrow --format csv > branches.csv
```

### Examples

**Before (v0.5):**
```bash
gat import matpower --m case9.m --output grid.arrow
gat pf dc grid.arrow --output flows.parquet --threads 4
gat inspect generators grid.arrow --format table
```

**After (v0.6):**
```bash
gat import matpower --m case9.m -o grid.arrow
gat pf dc grid.arrow -o flows.parquet -t 4
gat inspect generators grid.arrow -f json | jq '.[0]'
```

Both styles work—old flags remain as aliases for backward compatibility.
