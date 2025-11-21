use std::path::Path;

use anyhow::Result;

use crate::dataset::{import_dsgrid, import_pras};

pub fn handle_dsgrid(out: &Path) -> Result<()> {
    import_dsgrid(out)
}

pub fn handle_pras(path: &Path, out: &Path) -> Result<()> {
    import_pras(path, out)
}
