+++
title = "Installation Verification"
description = "Verify your GAT installation and test basic functionality"
slug = "install-verify"
weight = 5

[extra]
next_steps = [
  { title = "Quickstart Guide", description = "Run your first power flow analysis in 5 minutes", link = "/guide/quickstart/" },
  { title = "Power Flow Analysis", description = "Deep dive into power flow analysis options", link = "/guide/pf/" },
  { title = "Command Builder", description = "Interactively build commands", link = "/command-builder/" }
]
+++

After running the modular installer, verify that everything is working correctly with these quick tests.

## Quick Checks

### 1. Verify the Binary

```bash
gat --version
```

You should see output like:
```
gat 0.5.0
```

If you get "command not found", check that your shell has reloaded the PATH:
```bash
source ~/.profile  # or ~/.zshrc, ~/.bashrc depending on your shell
```

### 2. Check Installed Components

```bash
ls -la ~/.gat/bin/
```

You should see the `gat` executable and any other components you installed (e.g., solvers).

### 3. Verify Configuration Directory

```bash
ls -la ~/.gat/
```

This should show:
- `bin/` - Executable binaries
- `config/` - Configuration files
- `lib/` - Library components (if installed)
- `cache/` - Cached data

### 4. Test Basic Power Flow

The quickest test is a DC power flow on the test data included in the repository:

```bash
# Clone the repository for test data
git clone https://github.com/monistowl/gat.git
cd gat

# Import a MATPOWER case to Arrow format
gat import matpower --m test_data/matpower/case9.m -o test_grid.arrow

# Run DC power flow
gat pf dc test_grid.arrow --out flows.parquet

# Check the output
ls -lh flows.parquet
```

If this completes successfully, you should see a Parquet file with the results.

### 5. Inspect Results (Optional)

If you have `duckdb` or `polars` installed, you can inspect the results:

```bash
# With DuckDB
duckdb "SELECT * FROM 'flows.parquet' LIMIT 5"

# With Polars (Python)
python3 -c "import polars as pl; print(pl.read_parquet('flows.parquet'))"
```

## Troubleshooting

### "gat: command not found"

**Solution:** The binary isn't in your PATH. Reload your shell:
```bash
source ~/.profile
hash -r  # Clear command cache
gat --version
```

Or verify the binary exists:
```bash
~/.gat/bin/gat --version
```

### "error: Could not open file 'grid.arrow'"

**Solution:** Make sure you're in the GAT repository or use absolute paths:
```bash
cd /path/to/gat  # wherever you cloned the repository
gat import matpower --m test_data/matpower/case9.m -o /tmp/test_grid.arrow
gat pf dc /tmp/test_grid.arrow --out /tmp/flows.parquet
```

### "error: Solver X not found"

**Solution:** The solver wasn't installed. Run the installer again and select the solver you need:
```bash
bash ~/install-modular.sh
```

Then choose to install the desired solver (e.g., Clarabel, HiGHS, CBC).

### Installation hangs or fails

**Solution:** Check your internet connection and disk space:
```bash
# Check disk space in ~/.gat
df -h ~/.gat

# Check internet connectivity
curl -I https://github.com
```

If still stuck, download and run the installer directly:
```bash
curl -fsSL \
  https://github.com/monistowl/gat/releases/download/v0.5.0/install-modular.sh \
  -o install.sh
bash install.sh
```

## Next Steps

Once verified, you're ready to:

1. **Learn the basics** → [Quickstart Guide](/guide/quickstart/)
2. **Explore power flow analysis** → [Power Flow Analysis](/guide/pf/)
3. **Run optimization** → [Optimal Power Flow](/guide/opf/)
4. **Build commands interactively** → [Command Builder](/command-builder/)

## Getting Help

If something doesn't work:

1. Check the [FAQ](@/faq.md) for common issues
2. Review the [Quickstart Guide](/guide/quickstart/) for step-by-step examples
3. Open an issue on [GitHub](https://github.com/monistowl/gat/issues)
