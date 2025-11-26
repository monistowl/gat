# Windows Blockers (working log)

Purpose: running list of Windows-specific build/test issues discovered while enabling `x86_64-pc-windows-msvc`.

Format:
- Date (UTC)
- Command / context
- Failure signature (brief)
- Suspected cause
- Action/owner/status

## Entries
- 2025-11-26 — `cargo check -p gat-cli --target x86_64-pc-windows-gnu`
  - **Fail (resolved)**: `zstd-sys` missing `x86_64-w64-mingw32-gcc`.
  - **Fix**: installed `mingw-w64` cross toolchain; build now passes.

- 2025-11-26 — `cargo run -p gat-cli --target x86_64-pc-windows-gnu -- --help`
  - **Status**: runs under Wine after toolchain install; help output OK.

- 2025-11-26 — `cargo run -p gat-tui --target x86_64-pc-windows-gnu -- --help`
  - **Status (partial)**: cross-compilation now succeeds after gating `termios` behind `cfg(unix)` in `iocraft`; Windows path uses no-op raw-mode guard.
  - **Runtime check**: `wine target/x86_64-pc-windows-gnu/debug/gat-tui.exe --help` fails with `Invalid handle (os error 6)` — likely due to Wine console stdio handles.
  - **Next actions**: test on real Windows terminal or configure Wine console/`win32` subsystem; add Windows CI smoke once runtime confirmed.

## Triage rules
- Prefer minimal reproduction and crate/feature tags.
- If caused by external dep (CBC/OpenSSL), note install path and version.
- If blocked on upstream crate, open issue link when available.
