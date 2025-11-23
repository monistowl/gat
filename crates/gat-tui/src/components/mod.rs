// Core widget components using tuirealm/tui-realm-stdlib
//
// This module provides reusable wrapper components around tuirealm widgets,
// encapsulating state management, navigation, and interaction logic.

pub mod button;
pub mod container;
pub mod file_browser;
pub mod form;
pub mod input;
pub mod label;
pub mod list;
pub mod progress;
pub mod selector;
pub mod table;
pub mod tabs;
pub mod text;

pub use button::Button;
pub use container::Container;
pub use file_browser::{
    render_breadcrumb, render_file_entry, render_selection_info, render_tree_node,
    FileBrowserState, FileEntry, TreeEntry,
};
pub use form::{
    render_checkbox, render_form_section, render_form_section_with_fields, render_select,
    render_text_area, render_text_input, ConfigFormState, FormField, FormSection,
};
pub use input::InputWidget;
pub use label::Label;
pub use list::{ListItem, ListWidget};
pub use progress::{ProgressWidget, StatusLevel, StatusWidget};
pub use selector::{SelectOption, SelectWidget};
pub use table::{Column, TableRow, TableWidget};
pub use tabs::{Tab, TabsWidget};
pub use text::{ParagraphWidget, TextWidget};
