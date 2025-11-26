//! Export metadata helpers shared by format-specific serializers.
use crate::arrow_manifest::{ArrowManifest, SourceInfo};
use chrono::{DateTime, Utc};

/// Metadata extracted from the Arrow manifest to annotate exported files.
#[derive(Debug, Clone)]
pub struct ExportMetadata {
    pub source: Option<SourceInfo>,
    pub created_at: Option<DateTime<Utc>>,
    pub gat_version: Option<String>,
}

impl ExportMetadata {
    pub fn from_manifest(manifest: &ArrowManifest) -> Self {
        Self {
            source: manifest.source.clone(),
            created_at: Some(manifest.created_at),
            gat_version: Some(manifest.gat_version.clone()),
        }
    }

    pub fn source_description(&self) -> Option<String> {
        self.source.as_ref().map(|source| {
            format!(
                "{} ({}) hash {}",
                source.file, source.format, source.file_hash
            )
        })
    }

    pub fn creation_timestamp(&self) -> Option<String> {
        self.created_at
            .as_ref()
            .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    }

    pub fn gat_version(&self) -> Option<&str> {
        self.gat_version.as_deref()
    }
}
