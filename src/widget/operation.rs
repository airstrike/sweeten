//! Widget-agnostic focus operations.
//!
//! This module provides sweetened versions of focus operations that return
//! the [`widget::Id`] of the newly focused widget.
//!
//! # Example
//! ```no_run
//! # use iced_core::widget;
//! use sweeten::widget::operation;
//!
//! # #[derive(Debug, Clone)]
//! # enum Message { FocusedId(widget::Id), FocusNext }
//! # fn update(message: Message) -> iced_runtime::Task<Message> {
//! // Focus the next widget and get its ID:
//! operation::focus_next().map(Message::FocusedId)
//! # }
//! ```

use crate::core::widget;
use crate::core::widget::operation;
use iced_runtime::Task;

/// Produces a [`Task`] that focuses the next focusable widget
/// and returns the [`widget::Id`] of the newly focused widget.
///
/// This is a sweetened version of [`iced_runtime::widget::operation::focus_next`]
/// that tells you which widget received focus.
///
/// Use `.discard()` if you don't need the ID, or `.map(|id| ...)` to use it.
pub fn focus_next() -> Task<widget::Id> {
    iced_runtime::widget::operation::focus_next().chain(
        iced_runtime::task::widget(operation::focusable::find_focused()),
    )
}

/// Produces a [`Task`] that focuses the previous focusable widget
/// and returns the [`widget::Id`] of the newly focused widget.
///
/// This is a sweetened version of [`iced_runtime::widget::operation::focus_previous`]
/// that tells you which widget received focus.
///
/// Use `.discard()` if you don't need the ID, or `.map(|id| ...)` to use it.
pub fn focus_previous() -> Task<widget::Id> {
    iced_runtime::widget::operation::focus_previous().chain(
        iced_runtime::task::widget(operation::focusable::find_focused()),
    )
}

/// Produces a [`Task`] that focuses the widget with the given [`widget::Id`].
///
/// Re-exported from [`iced_runtime::widget::operation::focus`].
pub use iced_runtime::widget::operation::focus;
