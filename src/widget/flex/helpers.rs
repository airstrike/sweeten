//! Free-function constructors for the `flex` namespace.
//!
//! These mirror the `iced::widget::*` helper pattern: each free
//! function is a thin alias for the corresponding builder constructor
//! on the underlying widget. They live inside `widget::flex` so they
//! don't collide with the existing `crate::row` / `crate::column`
//! helpers — users opt into flex by reaching for `flex::row`,
//! `flex::column`, or `flex::flex(elem)`.

use crate::core::Element;

use super::child::FlexChild;
use super::column::Column;
use super::row::Row;

/// Wraps an [`Element`] in a [`FlexChild`] with CSS-default flex
/// properties.
///
/// The returned [`FlexChild`] can then be customised with
/// [`FlexChild::grow`], [`FlexChild::shrink`], [`FlexChild::basis`],
/// or [`FlexChild::align_self`] before being pushed into a flex
/// [`Row`] or [`Column`].
pub fn flex<'a, E, Message, Theme, Renderer>(
    content: E,
) -> FlexChild<'a, Message, Theme, Renderer>
where
    E: Into<Element<'a, Message, Theme, Renderer>>,
{
    FlexChild::new(content)
}

/// Creates a new flex [`Row`] containing the given plain children.
///
/// Each child enters the row as a [`FlexChild`] with CSS-default
/// properties. Use [`flex`] to opt a specific child into custom flex
/// behaviour.
pub fn row<'a, Message, Theme, Renderer>(
    children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
) -> Row<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    Row::with_children(children)
}

/// Creates a new flex [`Column`] containing the given plain children.
///
/// Each child enters the column as a [`FlexChild`] with CSS-default
/// properties. Use [`flex`] to opt a specific child into custom flex
/// behaviour.
pub fn column<'a, Message, Theme, Renderer>(
    children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
) -> Column<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    Column::with_children(children)
}
