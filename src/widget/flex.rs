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
//! # What's here today
//!
//! Phase 1 ships the pure layout engine, the property types, and the
//! alignment vocabulary. The `Row` / `Column` widgets and their helper
//! macros come in a follow-up phase. Public surface today:
//!
//! - [`Axis`] — main-axis selector
//! - [`AlignItems`] — container-level cross-axis alignment
//! - [`AlignSelf`] — per-child cross-axis override
//! - [`Justify`] — main-axis distribution
//! - [`FlexChild`] — element wrapper carrying flex properties
//!
//! [`widget::row`]: mod@crate::widget::row
//! [`widget::column`]: mod@crate::widget::column

pub mod alignment;
// Phase 1 ships the engine and child wrapper standalone; Phase 2 wires
// the consumer widgets on top, so engine helpers and the
// `resolved_properties` accessor have no internal callers yet.
#[allow(dead_code)]
mod child;
#[allow(dead_code)]
mod engine;

pub use alignment::{AlignItems, AlignSelf, Axis, Justify};
pub use child::FlexChild;
