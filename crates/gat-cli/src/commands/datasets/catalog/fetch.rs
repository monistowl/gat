use std::path::Path;

use anyhow::Result;

use crate::dataset::fetch_public_dataset;

pub fn handle(id: &str, out: Option<&String>, extract: bool, force: bool) -> Result<()> {
    fetch_public_dataset(id, out.as_deref().map(Path::new), extract, force).map(|_| ())
}
