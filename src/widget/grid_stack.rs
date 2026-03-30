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
//! - [`engine`] — The core layout engine (pure math, no iced dependency)
//! - [`item_id`] — The [`ItemId`] newtype for identifying grid items
//! - [`state`] — User-facing [`State`] that pairs the engine with user data
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
//!     Content::new(text(*label))
//!         .title_bar(TitleBar::new(text("Title")).padding(5))
//! })
//! .spacing(10)
//! .on_click(Message::Clicked)
//! .on_move(Message::Moved)
//! .on_resize(Message::Resized);
//! ```

pub mod content;
pub mod engine;
pub mod item_id;
pub mod state;
pub mod title_bar;
mod widget;

pub use content::Content;
pub use engine::GridEngine;
pub use item_id::ItemId;
pub use state::State;
pub use title_bar::TitleBar;
pub use widget::{
    Catalog, CellHeight, GridStack, Highlight, MoveEvent, ResizeEvent, Style,
    StyleFn,
};
