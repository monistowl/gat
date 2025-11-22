// Core widget components using tuirealm/tui-realm-stdlib
//
// This module provides reusable wrapper components around tuirealm widgets,
// encapsulating state management, navigation, and interaction logic.

pub mod container;
pub mod label;
pub mod button;
pub mod table;
pub mod list;
pub mod input;
pub mod text;
pub mod progress;
pub mod tabs;
pub mod selector;

pub use container::Container;
pub use label::Label;
pub use button::Button;
pub use table::{TableWidget, Column, TableRow};
pub use list::{ListWidget, ListItem};
pub use input::InputWidget;
pub use text::{TextWidget, ParagraphWidget};
pub use progress::{ProgressWidget, StatusWidget, StatusLevel};
pub use tabs::{TabsWidget, Tab};
pub use selector::{SelectWidget, SelectOption};
