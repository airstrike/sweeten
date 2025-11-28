//! Sweetened widgets for [`iced`].
//!
//! This module contains enhanced versions of common `iced` widgets. Each widget
//! is a drop-in replacement for its `iced` counterpart, with additional methods
//! for extended functionality.
//!
//! [`iced`]: https://github.com/iced-rs/iced

pub mod mouse_area;
pub mod overlay;
pub mod pick_list;
pub mod text_input;

pub use mouse_area::MouseArea;
pub use pick_list::PickList;
pub use text_input::TextInput;

// Re-export helper functions (same pattern as iced_widget)
pub use crate::helpers::*;
