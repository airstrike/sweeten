//! A GridStack-inspired layout engine for arranging items on a discrete grid.
//!
//! This module provides a coordinate-grid layout system inspired by
//! [GridStack.js](https://gridstackjs.com/). Items are placed on a grid with
//! a fixed number of columns and an unlimited (or capped) number of rows.
//! The engine handles collision resolution and optional vertical compaction
//! (gravity).
//!
//! Unlike iced's `pane_grid` which uses a binary tree of split ratios, this
//! system uses explicit integer `(x, y, w, h)` coordinates. This allows
//! arbitrary layouts that cannot be expressed as recursive binary splits,
//! such as L-shaped arrangements or items of varying sizes.
//!
//! # Architecture
//!
//! - [`engine`] — The [`Internal`] layout engine (pure math, no iced dependency)
//! - [`item_id`] — The [`ItemId`] newtype for identifying grid items
//! - [`state`] — User-facing [`State`] that pairs [`Internal`] with user data
//! - [`content`] — [`Content`] wrapper for item body + optional [`TitleBar`]
//! - [`title_bar`] — [`TitleBar`] for drag-handle and controls
//! - [`widget`](self::widget) — The [`GridStack`] widget implementation
//!
//! # Example
//!
//! ```ignore
//! use sweeten::widget::grid_stack::{self, GridStack, Content, TitleBar, State};
//! use iced::widget::text;
//!
//! let mut state: State<&str> = State::new(12);
//! let header = state.add(0, 0, 12, 1, "header");
//! let sidebar = state.add(0, 1, 3, 4, "sidebar");
//! let main = state.add(3, 1, 9, 4, "main");
//!
//! // In view:
//! let grid = GridStack::new(&state, |id, label| {
//!     grid_content(text(*label))
//!         .title_bar(title_bar(text("Title")).padding(5))
//! })
//! .spacing(10)
//! .on_action(Message::GridAction);
//! ```

pub mod content;
pub mod engine;
pub mod item_id;
pub mod state;
pub mod title_bar;
mod widget;

pub use content::Content;
pub use engine::{GridItem, Internal};
pub use item_id::ItemId;
pub use state::State;
pub use title_bar::TitleBar;
pub use widget::{
    Action, Catalog, CellHeight, DragPhase, GridStack, Highlight, ResizeGrip,
    Style, StyleFn,
};

use iced_widget::container;

use crate::core;
use crate::core::Element;

/// Creates a new [`Content`] with the provided body.
pub fn grid_content<'a, Message, Theme, Renderer>(
    body: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Content<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    Content::new(body)
}

/// Creates a new [`TitleBar`] with the given content.
pub fn title_bar<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> TitleBar<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    TitleBar::new(content)
}
