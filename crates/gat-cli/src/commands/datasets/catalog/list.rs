use anyhow::Result;

use crate::dataset::{list_public_datasets, PublicDatasetFilter};

pub fn handle(tag: Option<&String>, query: Option<&String>) -> Result<()> {
    let filter = PublicDatasetFilter {
        tag: tag.cloned(),
        query: query.cloned(),
    };
    list_public_datasets(&filter)
}
