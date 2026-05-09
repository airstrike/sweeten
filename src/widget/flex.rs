//! CSS-flex `Row` and `Column` widgets.
//!
//! This module hosts a parallel namespace to [`widget::row`] and
//! [`widget::column`] that implements the CSS Flexbox algorithm
//! faithfully — `justify-content`, `align-items`, `align-self`,
//! `flex-grow` / `flex-shrink` / `flex-basis`, direction reverse, gap,
//! and padding — while staying within `iced`'s O(n) layout budget
//! (≤ 2 layouts per child, no reflow loops).
//!
//! The existing `widget::row` and `widget::column` are untouched: this
//! is an opt-in parallel namespace, comparable to `tile_grid` and
//! `transition`.
//!
//! # API at a glance
//!
//! - [`Row`] / [`Column`] — the widgets, laid out along the horizontal
//!   or vertical main axis respectively.
//! - [`FlexChild`] — an [`Element`] paired with `flex-grow`,
//!   `flex-shrink`, `flex-basis`, and `align-self`. Plain elements
//!   enter via `From` with CSS defaults.
//! - [`flex`] — wraps any `Into<Element>` in a [`FlexChild`] so callers
//!   can chain `.grow(_)`, `.shrink(_)`, `.basis(_)`, `.align_self(_)`.
//! - [`row`] / [`column`] — free-function constructors that take any
//!   `IntoIterator<Item = Element>`.
//! - [`row!`] / [`column!`] — declarative macros mirroring iced's
//!   `row![...]` / `column![...]`. They live at the canonical path
//!   `widget::flex::{row, column}`; `#[macro_export]` also lands them
//!   at the crate root as [`crate::flex_row`] / [`crate::flex_column`].
//!
//! [`Element`]: crate::core::Element
//! [`widget::row`]: mod@crate::widget::row
//! [`widget::column`]: mod@crate::widget::column

pub mod alignment;
mod child;
pub mod column;
mod engine;
mod helpers;
pub mod row;

pub use alignment::{AlignItems, AlignSelf, Axis, Justify};
pub use child::FlexChild;
pub use column::Column;
pub use helpers::{column, flex, row};
pub use row::Row;

// Macro namespace — re-aliased from the crate-root macros into the
// canonical `widget::flex::{row, column}` paths. Rust's three-namespace
// rule (types / values / macros) lets `flex::row` simultaneously name
// the module, the helper function, and the macro — exactly the way
// `iced::widget::column` does for its own three-way collision.
#[doc(inline)]
pub use crate::{flex_column as column, flex_row as row};
