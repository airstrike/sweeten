//! Sweetened widgets for [`iced`].
//!
//! This module contains enhanced versions of common `iced` widgets. Each widget
//! is a drop-in replacement for its `iced` counterpart, with additional methods
//! for extended functionality.
//!
//! [`iced`]: https://github.com/iced-rs/iced

pub mod button;
pub mod column;
pub mod drag;
pub mod fit_text;
pub mod mouse_area;
pub mod operation;
pub mod overlay;
pub mod pick_list;
pub mod row;
pub mod table;
pub mod text_input;
pub mod tile_grid;
pub mod toggler;
pub mod transition;

pub use button::Button;
pub use column::Column;
pub use fit_text::FitText;
pub use mouse_area::MouseArea;
pub use pick_list::PickList;
pub use row::Row;
pub use table::Table;
pub use text_input::TextInput;
pub use tile_grid::TileGrid;
pub use toggler::Toggler;
pub use transition::Transition;

// Re-export helper functions
pub use crate::helpers::*;

pub use crate::{column, row};
