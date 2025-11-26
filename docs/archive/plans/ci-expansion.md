# CI Expansion Task List (Windows + WASM)

Use this as a checklist for workflow updates. Link PRs/issues as you complete items.

## Windows matrix
- [ ] Add `windows-latest` to `.github/workflows/build-matrix.yml` with PowerShell steps.
- [ ] Install deps via vcpkg: `openssl:x64-windows` (and optional CBC).
- [ ] Upload `.zip` artifacts named `gat-{variant}-windows-x86_64.zip`.
- [ ] Smoke: `./target\release\gat.exe --help` (or artifact binary) on runner.
- [ ] Cache keys per OS/target.

## WASM job
- [ ] Install target `rustup target add wasm32-wasi` in workflow helper.
- [ ] `cargo check --features wasm --target wasm32-wasi` (headless feature set).
- [ ] Upload blocker report on failure (append to `docs/plans/wasm-compat.md`).

## Fast PR workflow
- [ ] Add Windows build-only leg (headless, no solvers) to keep under 10m.
- [ ] Add WASM check leg (can be allow-fail initially, but report).

## Nightly/Release
- [ ] Ensure reusable matrix inherits Windows; artifact retention still 14/7 days.
- [ ] Verify artifact naming includes platform.

## Observability
- [ ] Save verbose build logs for Windows/WASM jobs.
- [ ] Add `ci/tools/report-blockers.sh` to summarize failing crates by target.

