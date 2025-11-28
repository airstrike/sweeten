//! # sweeten
//!
//! `sweeten` provides enhanced versions of common [`iced`] widgets with
//! additional functionality for more complex use cases. It aims to maintain
//! the simplicity and elegance of `iced` while offering "sweetened" variants
//! with extended capabilities.
//!
//! ## Widgets
//!
//! The following widgets are available in the [`widget`] module:
//!
//! - [`mouse_area`] — A container for capturing mouse events, with support for
//!   receiving the click position via [`on_press_with`].
//! - [`pick_list`] — A dropdown list of selectable options, with support for
//!   disabling items.
//! - [`text_input`] — A text input field, with support for [`on_focus`] and
//!   [`on_blur`] messages.
//!
//! ## Usage
//!
//! Import the widgets you need from `sweeten::widget`:
//!
//! ```no_run
//! use sweeten::widget::{mouse_area, pick_list, text_input};
//! # fn main() {}
//! ```
//!
//! The widgets are designed to be drop-in replacements for their `iced`
//! counterparts, with additional methods for the extended functionality.
//!
//! [`iced`]: https://github.com/iced-rs/iced
//! [`mouse_area`]: mod@widget::mouse_area
//! [`pick_list`]: mod@widget::pick_list
//! [`text_input`]: mod@widget::text_input
//! [`on_press_with`]: widget::mouse_area::MouseArea::on_press_with
//! [`on_focus`]: widget::text_input::TextInput::on_focus
//! [`on_blur`]: widget::text_input::TextInput::on_blur

mod helpers;
pub mod widget;

pub use helpers::*;

// Re-exports to mirror iced_widget structure (allows minimal diff for widgets)
use iced_core as core;
pub use iced_core::Theme;
pub use iced_widget::Renderer;
pub use iced_widget::{button, scrollable, text_editor};

// Re-export widget modules at crate level (mirrors iced_widget's structure)
pub use widget::overlay;
pub use widget::text_input;
