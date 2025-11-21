use anyhow::Result;

use crate::dataset::describe_public_dataset;

pub fn handle(id: &str) -> Result<()> {
    describe_public_dataset(id)
}
