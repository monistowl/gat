# Packaging & installers

GAT ships with helper scripts that build release binaries, package them into tarballs, and install them locally.

1. Run `scripts/package.sh` (requires `jq` + `cargo`) to build release artifacts and produce `dist/gat-<version>-<os>-<arch>.tar.gz`. The tarball contains the CLI (`gat-cli`), GUI helper (`gat-gui`), docs, and `scripts/install.sh`.

2. Extract the tarball and run `scripts/install.sh [prefix]` (defaults to `~/.local`). The script copies `gat-cli`/`gat-gui` to `prefix/bin` and marks them executable.

3. Use the `gat` CLI after installing to parse networks, run PF/OPF/SE, or launch the stub GUI. This mirrors the installer expectations noted in `../ROADMAP.md` (M10 packaging) and gives labs a repeatable, scriptable flow for deployment.
