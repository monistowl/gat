+++
title = "Packaging & Installers"
description = "Building release packages and installation scripts"
weight = 150
+++

# Packaging & installers

GAT includes helper scripts that build release binaries, package them, and install them locally.

1. Run `scripts/package.sh` (requires `jq` and `cargo`) to produce `dist/gat-<version>-<os>-<arch>.tar.gz`. The tarball bundles `gat-cli`, `gat-gui`, the docs folder, and `scripts/install.sh`.
2. Extract the tarball and run `scripts/install.sh [prefix]` (defaults to `~/.local`). It copies `gat-cli`/`gat-gui` to `prefix/bin` and marks them executable.
3. After installation, `gat` is ready to run PF/OPF/SE/TS workflows or launch the stub GUI documented in `docs/guide/gui.md`.

This mirrors the installer expectations from `docs/ROADMAP.md` (M10 packaging) and gives labs a repeatable, scriptable deployment flow.
