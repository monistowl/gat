#[cfg(not(target_arch = "wasm32"))]
pub mod importers;
#[cfg(not(target_arch = "wasm32"))]
pub mod sources;
pub mod validate;

// For wasm builds we stub IO; web demo should provide data from host/JS.
#[cfg(target_arch = "wasm32")]
pub mod wasm_stub {
    use anyhow::{bail, Result};

    pub fn load_csv_stub(_data: &str) -> Result<()> {
        bail!("gat-io wasm build: CSV/Parquet IO not available in wasm stub")
    }
}
