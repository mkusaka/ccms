//! Custom hooks for the interactive interface

pub mod use_debounce;
pub mod use_terminal_events;
pub mod use_search;
pub mod use_keyboard_navigation;
pub mod use_clipboard;
pub mod use_memo;
pub mod use_virtual_scroll;

pub use use_debounce::{use_debounce, use_debounced_value, use_debounced_callback, use_debounced_search};
pub use use_terminal_events::{use_terminal_events, is_quit_key, is_escape_key, is_enter_key};
pub use use_search::{use_search, UseSearchResult};
pub use use_keyboard_navigation::use_keyboard_navigation;
pub use use_clipboard::{use_clipboard, copy_to_clipboard};
pub use use_memo::{use_memo, use_callback, use_ref, use_memo_list};
pub use use_virtual_scroll::{use_virtual_scroll, use_virtual_list, handle_virtual_scroll, VirtualScrollConfig, VirtualScrollState, ScrollDirection};