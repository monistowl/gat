//! Format detection and unified import interface.
//!
//! This module provides a `Format` enum that unifies format detection and parsing
//! across all supported power system file formats.

use std::path::Path;

use anyhow::Result;

use crate::helpers::ImportResult;

/// Supported import formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// MATPOWER .m case files
    Matpower,
    /// PSS/E RAW files
    Psse,
    /// CIM RDF/XML files
    Cim,
    /// pandapower JSON files
    Pandapower,
    /// PowerModels.jl JSON files
    PowerModels,
}

/// Confidence level for format detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Extension matches but content not verified
    Low,
    /// Extension and some content markers match
    Medium,
    /// Strong content markers confirm format
    High,
}

impl Format {
    /// All supported formats.
    pub const ALL: &'static [Format] = &[
        Format::Matpower,
        Format::Psse,
        Format::Cim,
        Format::Pandapower,
        Format::PowerModels,
    ];

    /// Expected file extensions for this format.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Format::Matpower => &["m", "mat", "matpower", "case"],
            Format::Psse => &["raw"],
            Format::Cim => &["rdf", "xml"],
            Format::Pandapower => &["json"],
            Format::PowerModels => &["json"],
        }
    }

    /// Human-readable format name.
    pub fn friendly_name(&self) -> &'static str {
        match self {
            Format::Matpower => "MATPOWER case",
            Format::Psse => "PSS/E RAW",
            Format::Cim => "CIM RDF/XML",
            Format::Pandapower => "pandapower JSON",
            Format::PowerModels => "PowerModels.jl JSON",
        }
    }

    /// CLI subcommand name for this format.
    pub fn command_name(&self) -> &'static str {
        match self {
            Format::Matpower => "matpower",
            Format::Psse => "psse",
            Format::Cim => "cim",
            Format::Pandapower => "pandapower",
            Format::PowerModels => "powermodels",
        }
    }

    /// Detect format from file path and optionally content.
    ///
    /// Returns the detected format and confidence level, or None if no format matches.
    pub fn detect(path: &Path) -> Option<(Format, Confidence)> {
        // First try extension-based detection
        let ext = path.extension()?.to_str()?.to_lowercase();

        // Check each format's extensions
        for format in Self::ALL {
            if format
                .extensions()
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&ext))
            {
                // For ambiguous extensions, try content sniffing
                let confidence = match format {
                    Format::Cim | Format::Pandapower | Format::PowerModels => {
                        // XML and JSON are ambiguous - need content check
                        if let Ok(confidence) = format.sniff_content(path) {
                            confidence
                        } else {
                            Confidence::Low
                        }
                    }
                    _ => Confidence::Medium, // Extension is fairly specific
                };
                return Some((*format, confidence));
            }
        }

        None
    }

    /// Sniff file content to verify format.
    fn sniff_content(&self, path: &Path) -> Result<Confidence> {
        // Read first ~4KB for sniffing
        let content = std::fs::read_to_string(path)
            .map(|s| s.chars().take(4096).collect::<String>())
            .unwrap_or_default();

        match self {
            Format::Matpower => {
                // Look for "function mpc = " or "mpc.baseMVA"
                if content.contains("function mpc") || content.contains("mpc.baseMVA") {
                    Ok(Confidence::High)
                } else if content.contains("mpc.") {
                    Ok(Confidence::Medium)
                } else {
                    Ok(Confidence::Low)
                }
            }
            Format::Psse => {
                // RAW files typically start with case ID and system base MVA
                // First line often contains version info
                let first_line = content.lines().next().unwrap_or("");
                if first_line.contains("PSS") || first_line.contains("RAW") {
                    Ok(Confidence::High)
                } else {
                    // Check for typical RAW structure: numbers in first lines
                    Ok(Confidence::Medium)
                }
            }
            Format::Cim => {
                // Look for CIM namespace or rdf:RDF
                if content.contains("cim:") || content.contains("rdf:RDF") {
                    Ok(Confidence::High)
                } else if content.contains("<?xml") {
                    Ok(Confidence::Low)
                } else {
                    Ok(Confidence::Low)
                }
            }
            Format::Pandapower => {
                // Look for pandapower markers in JSON
                if content.contains("pandapowerNet") || content.contains("\"_module\"") {
                    Ok(Confidence::High)
                } else if content.starts_with('{') {
                    Ok(Confidence::Low)
                } else {
                    Ok(Confidence::Low)
                }
            }
            Format::PowerModels => {
                // Look for PowerModels markers: baseMVA at root level with bus/gen/branch
                if content.contains("\"baseMVA\"") && content.contains("\"bus\"") {
                    // Check it's not pandapower (which has nested structure)
                    if !content.contains("pandapowerNet") && !content.contains("\"_module\"") {
                        Ok(Confidence::High)
                    } else {
                        Ok(Confidence::Low)
                    }
                } else if content.starts_with('{') && content.contains("\"bus\"") {
                    Ok(Confidence::Medium)
                } else {
                    Ok(Confidence::Low)
                }
            }
        }
    }

    /// Parse a file in this format, returning an ImportResult with diagnostics.
    pub fn parse(&self, path: &str) -> Result<ImportResult> {
        match self {
            Format::Matpower => super::parse_matpower(path),
            Format::Psse => super::parse_psse(path),
            Format::Cim => super::parse_cim(path),
            Format::Pandapower => super::parse_pandapower(path),
            Format::PowerModels => super::parse_powermodels(path),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.friendly_name())
    }
}

impl std::str::FromStr for Format {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "matpower" | "m" => Ok(Format::Matpower),
            "psse" | "raw" => Ok(Format::Psse),
            "cim" | "rdf" | "cgmes" => Ok(Format::Cim),
            "pandapower" | "pp" => Ok(Format::Pandapower),
            "powermodels" | "pm" | "julia" => Ok(Format::PowerModels),
            _ => anyhow::bail!(
                "Unknown format: {}. Supported: matpower, psse, cim, pandapower, powermodels",
                s
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_matpower() {
        let path = Path::new("case14.m");
        let (format, _) = Format::detect(path).unwrap();
        assert_eq!(format, Format::Matpower);
    }

    #[test]
    fn test_detect_psse() {
        let path = Path::new("ieee14.raw");
        let (format, _) = Format::detect(path).unwrap();
        assert_eq!(format, Format::Psse);
    }

    #[test]
    fn test_detect_pandapower() {
        let path = Path::new("network.json");
        let (format, _) = Format::detect(path).unwrap();
        assert_eq!(format, Format::Pandapower);
    }

    #[test]
    fn test_format_from_str() {
        assert_eq!("matpower".parse::<Format>().unwrap(), Format::Matpower);
        assert_eq!("psse".parse::<Format>().unwrap(), Format::Psse);
        assert_eq!("cim".parse::<Format>().unwrap(), Format::Cim);
        assert_eq!("pandapower".parse::<Format>().unwrap(), Format::Pandapower);
    }

    #[test]
    fn test_extensions() {
        assert!(Format::Matpower.extensions().contains(&"m"));
        assert!(Format::Psse.extensions().contains(&"raw"));
        assert!(Format::Cim.extensions().contains(&"rdf"));
        assert!(Format::Pandapower.extensions().contains(&"json"));
    }
}
