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
//! `transition`. The pitch is *iced ergonomics, CSS faithfulness* —
//! you keep iced's builder style and `Element` conversions, and you
//! get the CSS spelling for every property that has one.
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
//! - [`row`][row()] / [`column`][column()] — free-function constructors
//!   that take any `IntoIterator<Item = Element>`.
//! - [`row!`] / [`column!`] — declarative macros mirroring iced's
//!   `row![...]` / `column![...]`. They live at the canonical path
//!   `widget::flex::{row, column}`; `#[macro_export]` also lands them
//!   at the crate root as [`crate::flex_row`] / [`crate::flex_column`].
//!
//! # `widget::row` vs. `widget::flex::row`
//!
//! Both ship in `sweeten` and they're complementary, not competing:
//!
//! | | [`widget::row`] | `widget::flex::row` |
//! |---|---|---|
//! | **Origin** | upstream `iced_widget::row` + drag-and-drop | net-new, sweeten-only |
//! | **Layout** | `iced_core::layout::flex::resolve` | this module's CSS-flex engine |
//! | **`justify-content`** | not exposed (insert `Space::with_width(Fill)` to fake) | first-class — [`Row::justify`] |
//! | **Per-item grow / shrink / basis** | only `Length::Fill` / `FillPortion(_)` | [`FlexChild::grow`] / [`FlexChild::shrink`] / [`FlexChild::basis`] |
//! | **`align-self` override** | no | [`FlexChild::align_self`] |
//! | **`flex-direction: row-reverse`** | no | [`Row::reverse`] |
//! | **Drag-and-drop reorder** | yes — [`row::Row::on_drag`][drag_row] | not in v1 |
//! | **`flex-wrap`** | yes — [`row::Row::wrap`][wrap_row] | not in v1 |
//!
//! Reach for [`widget::row`] when you want drag-and-drop or wrap.
//! Reach for `widget::flex::row` when you want CSS-flex distribution
//! semantics or per-item ratios. They're separate import paths so you
//! can mix them in the same view freely.
//!
//! # CSS-name → builder mapping
//!
//! | CSS                                  | builder                                         |
//! |--------------------------------------|-------------------------------------------------|
//! | `justify-content`                    | [`Row::justify`] / [`Column::justify`]          |
//! | `align-items`                        | [`Row::align`] / [`Column::align`]              |
//! | `align-self`                         | [`FlexChild::align_self`]                       |
//! | `flex-grow`                          | [`FlexChild::grow`]                             |
//! | `flex-shrink`                        | [`FlexChild::shrink`]                           |
//! | `flex-basis`                         | [`FlexChild::basis`]                            |
//! | `gap`                                | [`Row::gap`] / [`Column::gap`]                  |
//! | `padding`                            | [`Row::padding`] / [`Column::padding`]          |
//! | `flex-direction: row-reverse`        | [`Row::reverse`]                                |
//! | `flex-direction: column-reverse`    | [`Column::reverse`]                             |
//!
//! [`Row::align`] accepts both the explicit [`AlignItems`] enum and
//! `iced`'s canonical [`Alignment`] consts (`iced::Start`,
//! `iced::Center`, `iced::End`) via the `Into<AlignItems>` impl. The
//! one variant `iced::Alignment` doesn't carry — `Stretch` — is reached
//! via the explicit enum.
//!
//! # Performance budget
//!
//! At most **two layout calls per child**, and often only one:
//!
//! - Items with [`FlexChild::basis`] set to a pixel value skip the
//!   basis-measurement pass — the engine takes the supplied basis
//!   directly. They resolve in **one** layout call (the final-sizing
//!   pass).
//! - Items with `Length::Fill` and the default `Basis::Auto` are
//!   treated as `(grow=fill_factor, basis=0)`, matching iced's
//!   existing fluid-item behaviour. They also resolve in **one**
//!   layout call.
//! - Items with intrinsic sizing (`Length::Shrink` + `Basis::Auto`)
//!   need a basis pass *and* a final pass — **two** calls.
//!
//! No iterative reflow. No watchdog. Same upper bound as iced's own
//! `flex::resolve` (`iced/core/src/layout/flex.rs`).
//!
//! # The `flex::row` / `flex::column` three-namespace coexistence
//!
//! `flex::row` simultaneously names a **module** (the type namespace),
//! a **free function** (the value namespace), and a **macro** (the
//! macro namespace). Rust keeps these three namespaces separate, so
//! all three coexist at the same path:
//!
//! | Path used as…                          | Resolves to                          |
//! |----------------------------------------|--------------------------------------|
//! | `flex::row` in a `use` import          | the [`row`][mod@row] module          |
//! | `flex::row(…)` call expression         | the [`row()`][row()] function        |
//! | `flex::row![…]` macro invocation       | the [`row!`] declarative macro       |
//! | `flex::Row` type path                  | the [`Row`] widget                   |
//!
//! Same for `column`. The macros are also re-exported at the crate
//! root via `#[macro_export]` as [`crate::flex_row`] /
//! [`crate::flex_column`] — that's a side effect of how
//! `#[macro_export]` works (macros always land at the consuming
//! crate's root) rather than a separate API choice. The canonical
//! path remains `widget::flex::row!` / `widget::flex::column!`.
//!
//! [`Element`]: crate::core::Element
//! [`Alignment`]: crate::core::Alignment
//! [`widget::row`]: mod@crate::widget::row
//! [`widget::column`]: mod@crate::widget::column
//! [drag_row]: crate::widget::row::Row::on_drag
//! [wrap_row]: crate::widget::row::Row::wrap

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

/// Internal engine API exposed for benchmarking only.
///
/// This module is `#[doc(hidden)]` and not part of the stable surface;
/// it exists so the `flex_vs_iced` benchmark in `benches/` can drive
/// [`engine::resolve`] without going through the iced widget tree. Do
/// not depend on it from application code — it may change or vanish
/// without notice.
#[doc(hidden)]
pub mod __bench {
    pub use super::child::{Basis, Properties};
    pub use super::engine::{
        cross_offset, justify_offsets, resolve, solve_main_sizes,
    };
}
