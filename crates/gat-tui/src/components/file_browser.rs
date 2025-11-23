// File browser component system
//
// Provides two-tier component architecture:
// 1. Stateless rendering functions - pure functions that render directory listings
// 2. Optional state wrapper - manages browsing state and navigation

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A single file entry in a directory listing
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: SystemTime,
    pub path: PathBuf,
}

/// A tree node for hierarchical directory display
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub children_count: usize,
    pub path: PathBuf,
}

/// State management for interactive file browsing
#[derive(Debug)]
pub struct FileBrowserState {
    pub root_path: PathBuf,
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub tree_expanded: HashSet<PathBuf>,
    pub selected_index: usize,
    pub show_details: bool,
    pub filter: Option<String>, // e.g., "*.csv"
}

impl FileBrowserState {
    /// Initialize a file browser from a directory path
    pub fn new(root_path: PathBuf) -> anyhow::Result<Self> {
        let mut state = Self {
            root_path: root_path.clone(),
            current_path: root_path.clone(),
            entries: Vec::new(),
            tree_expanded: HashSet::new(),
            selected_index: 0,
            show_details: true,
            filter: None,
        };

        state.refresh_entries()?;
        Ok(state)
    }

    /// Navigate to a specific directory
    pub fn navigate_to(&mut self, path: &Path) -> anyhow::Result<()> {
        if path.is_dir() {
            self.current_path = path.to_path_buf();
            self.selected_index = 0;
            self.refresh_entries()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Path is not a directory"))
        }
    }

    /// Go to parent directory
    pub fn parent_directory(&mut self) -> anyhow::Result<()> {
        let parent = self.current_path.parent().map(|p| p.to_path_buf());
        if let Some(parent_path) = parent {
            self.navigate_to(&parent_path)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Already at root directory"))
        }
    }

    /// Select the next entry
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
        }
    }

    /// Select the previous entry
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Enter selected directory or confirm file selection
    pub fn enter_selected(&mut self) -> anyhow::Result<Option<PathBuf>> {
        let entry = self.entries.get(self.selected_index).cloned();
        if let Some(entry) = entry {
            if entry.is_dir {
                self.navigate_to(&entry.path)?;
                Ok(None)
            } else {
                Ok(Some(entry.path))
            }
        } else {
            Err(anyhow::anyhow!("No entry selected"))
        }
    }

    /// Get the currently selected file (if any)
    pub fn get_selected_file(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    /// Toggle expansion state of a directory
    pub fn toggle_expanded(&mut self, path: &Path) {
        if self.tree_expanded.contains(path) {
            self.tree_expanded.remove(path);
        } else {
            self.tree_expanded.insert(path.to_path_buf());
        }
    }

    /// Apply filter pattern (e.g., "*.csv")
    pub fn apply_filter(&mut self, pattern: &str) {
        self.filter = Some(pattern.to_string());
        let _ = self.refresh_entries();
    }

    /// Clear filter
    pub fn clear_filter(&mut self) {
        self.filter = None;
        let _ = self.refresh_entries();
    }

    /// Get breadcrumb path components
    pub fn get_breadcrumb(&self) -> Vec<String> {
        self.current_path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    }

    /// Refresh the entry list from current directory
    fn refresh_entries(&mut self) -> anyhow::Result<()> {
        self.entries.clear();

        if !self.current_path.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.current_path)? {
            let entry = entry?;
            let path = entry.path();
            let is_dir = entry.file_type()?.is_dir();

            // Skip if filter is set and doesn't match
            if let Some(ref filter) = self.filter {
                if !is_dir && !matches_filter(&path, filter) {
                    continue;
                }
            }

            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();

            self.entries.push(FileEntry {
                name,
                is_dir,
                size: metadata.len(),
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                path,
            });
        }

        // Sort: directories first, then alphabetically
        self.entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(())
    }
}

/// Check if a path matches a filter pattern
fn matches_filter(path: &Path, pattern: &str) -> bool {
    if let Some(file_name) = path.file_name() {
        let file_name = file_name.to_string_lossy();
        // Simple wildcard matching: *.ext matches files ending with .ext
        if pattern.starts_with('*') {
            let ext = &pattern[1..];
            file_name.ends_with(ext)
        } else {
            file_name.contains(pattern)
        }
    } else {
        false
    }
}

// ============================================================================
// STATELESS RENDERING FUNCTIONS
// ============================================================================

/// Render a single directory tree node
pub fn render_tree_node(
    name: &str,
    is_dir: bool,
    is_selected: bool,
    is_expanded: bool,
    indent_level: usize,
) -> String {
    let indent = "  ".repeat(indent_level);
    let indicator = if is_selected { "▶" } else { " " };
    let icon = if is_dir {
        if is_expanded {
            "▼"
        } else {
            "▶"
        }
    } else {
        "─"
    };

    let prefix = if is_selected {
        format!("{} {} ", indicator, icon)
    } else {
        format!("  {} ", icon)
    };

    format!("{}{}{}", indent, prefix, name)
}

/// Render a breadcrumb path
pub fn render_breadcrumb(path_components: &[String], separator: &str) -> String {
    path_components.join(separator)
}

/// Render file list entry
pub fn render_file_entry(
    name: &str,
    is_dir: bool,
    is_selected: bool,
    show_details: bool,
    size: u64,
) -> String {
    let indicator = if is_selected { "▶" } else { " " };
    let icon = if is_dir { "📁" } else { "📄" };

    if show_details && !is_dir {
        let size_str = format_size(size);
        format!("{} {} {} ({})", indicator, icon, name, size_str)
    } else {
        format!("{} {} {}", indicator, icon, name)
    }
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_idx])
}

/// Render selection info footer
pub fn render_selection_info(
    current_path: &str,
    selected_file: Option<&str>,
    file_count: usize,
) -> String {
    let selected_info = selected_file
        .map(|f| format!(" Selected: {}", f))
        .unwrap_or_default();

    format!(
        "Path: {} | Files: {}{}",
        current_path, file_count, selected_info
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_filter() {
        let path = PathBuf::from("data.csv");
        assert!(matches_filter(&path, "*.csv"));
        assert!(!matches_filter(&path, "*.json"));

        let path = PathBuf::from("file.txt");
        assert!(matches_filter(&path, "*.txt"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
    }

    #[test]
    fn test_render_tree_node() {
        let result = render_tree_node("datasets", true, true, false, 0);
        assert!(result.contains("▶"));
        assert!(result.contains("datasets"));
    }
}
