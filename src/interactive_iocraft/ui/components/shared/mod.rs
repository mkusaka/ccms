//! Shared components used across views

pub mod search_bar;
pub mod result_list;
pub mod status_bar;
pub mod text_input;
// pub mod modal;  // Not used currently

pub use search_bar::{SearchBar, SearchBarProps};
pub use result_list::{ResultList, ResultListProps};
pub use status_bar::StatusBar;
pub use text_input::{AdvancedTextInput, AdvancedTextInputProps};
// pub use modal::Modal;  // Not used currently