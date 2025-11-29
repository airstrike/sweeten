#![warn(missing_docs)]
// This crate contains modifications of widgets from [`iced`].
//
// [`iced`]: https://github.com/iced-rs/iced
//
// Copyright 2019 Héctor Ramón, Iced contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

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
//! - [`column`] — Distribute content vertically, with support for drag-and-drop
//!   reordering via [`on_drag`](widget::column::Column::on_drag).
//! - [`mouse_area`] — A container for capturing mouse events where all handlers
//!   receive the cursor position as a [`Point`].
//! - [`pick_list`] — A dropdown list of selectable options, with support for
//!   disabling items.
//! - [`row`] — Distribute content horizontally, with support for drag-and-drop
//!   reordering via [`on_drag`](widget::row::Row::on_drag).
//! - [`text_input`] — A text input field, with support for [`on_focus`] and
//!   [`on_blur`] messages.
//!
//! ## Usage
//!
//! Import the widgets you need from `sweeten::widget`:
//!
//! ```no_run
//! use sweeten::widget::{column, mouse_area, pick_list, row, text_input};
//! # fn main() {}
//! ```
//!
//! The widgets are designed to be drop-in replacements for their `iced`
//! counterparts, with additional methods for the extended functionality.
//!
//! [`iced`]: https://github.com/iced-rs/iced
//! [`column`]: mod@widget::column
//! [`mouse_area`]: mod@widget::mouse_area
//! [`pick_list`]: mod@widget::pick_list
//! [`row`]: mod@widget::row
//! [`text_input`]: mod@widget::text_input
//! [`Point`]: crate::core::Point
//! [`on_focus`]: widget::text_input::TextInput::on_focus
//! [`on_blur`]: widget::text_input::TextInput::on_blur

mod helpers;
pub mod widget;

pub use helpers::*;

// Re-exports to mirror iced_widget structure (allows minimal diff for widgets)
pub use iced_core as core;
pub use iced_core::Theme;
pub use iced_widget::Renderer;
pub use iced_widget::{button, scrollable, text_editor};

// Re-export widget modules at crate level (mirrors iced_widget's structure)
pub use widget::overlay;
pub use widget::text_input;
