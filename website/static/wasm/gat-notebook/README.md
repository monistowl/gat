# GAT Notebook WASM bundle (drop-in)

This folder is where the real `gat-notebook` WebAssembly build should be published for the website demos.

## Expected artifacts
- `notebook.js` (wasm-bindgen JS shim from `wasm-pack build --target web`)
- `notebook_bg.wasm` (optimized WASM binary)
- optional: `package.json`/`README.md` emitted by `wasm-pack`

### Hooking into the homepage demo
The homepage expects to call a simple export when visitors press the **Run… in WASM** button. If your wasm-bindgen output exposes any of the following function names they will be detected automatically:

- `run_demo` (preferred: accepts a notebook slug such as `"ac-opf"`)
- `run_cell`
- `main` / `start` (fallback)

Wire a small JS shim in `notebook.js` that imports `notebook_bg.wasm`, initializes the module, and re-exports an async `run_demo(slug: string)` function. The front-end will locate it, update the status badge, and use the WASM-backed path instead of the mock timeline.

## Local build attempt
A `cargo build -p gat-notebook --target wasm32-unknown-unknown` run was attempted, but the toolchain could not download the `wasm32-unknown-unknown` standard library in this offline container. Install the target where network access is available and rerun the build.

Example build script (runs from repo root):

```bash
rustup target add wasm32-unknown-unknown
wasm-pack build crates/gat-notebook --target web --out-dir website/static/wasm/gat-notebook --release
```

When the build succeeds, ship the generated files here and the homepage will pick them up automatically.
