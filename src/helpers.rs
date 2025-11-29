//! Helper functions to create widgets.

use crate::core;
use crate::core::Element;
use crate::overlay::menu;
use crate::widget::MouseArea;
use crate::widget::column::{self, Column};
use crate::widget::pick_list::{self, PickList};
use crate::widget::row::{self, Row};
use crate::widget::text_input::{self, TextInput};

use std::borrow::Borrow;

/// Creates a [`Column`] with the given children.
///
/// Columns distribute their children vertically.
#[macro_export]
macro_rules! column {
    () => (
        $crate::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::widget::Column::with_children([$($crate::core::Element::from($x)),+])
    );
}

/// Creates a [`Row`] with the given children.
///
/// Rows distribute their children horizontally.
#[macro_export]
macro_rules! row {
    () => (
        $crate::widget::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::widget::Row::with_children([$($crate::core::Element::from($x)),+])
    );
}

/// Creates a new [`Row`] with the given children.
pub fn row<'a, Message, Theme, Renderer>(
    children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
) -> Row<'a, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
    Theme: row::Catalog,
{
    Row::with_children(children)
}

/// Creates a new [`Column`] with the given children.
pub fn column<'a, Message, Theme, Renderer>(
    children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
) -> Column<'a, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
    Theme: column::Catalog,
{
    Column::with_children(children)
}

/// Creates a new [`TextInput`].
///
/// This is a sweetened version of [`iced`'s `text_input`] with support for
/// [`on_focus`] and [`on_blur`] messages.
///
/// [`iced`'s `text_input`]: https://docs.iced.rs/iced/widget/text_input/index.html
/// [`on_focus`]: TextInput::on_focus
/// [`on_blur`]: TextInput::on_blur
pub fn text_input<'a, Message, Theme, Renderer>(
    placeholder: &str,
    value: &str,
) -> TextInput<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: text_input::Catalog + 'a,
    Renderer: core::text::Renderer,
{
    TextInput::new(placeholder, value)
}

/// Creates a new [`PickList`].
///
/// This is a sweetened version of [`iced`'s `pick_list`] with support for
/// disabling items in the dropdown via [`disabled`].
///
/// [`iced`'s `pick_list`]: https://docs.iced.rs/iced/widget/pick_list/index.html
/// [`disabled`]: PickList::disabled
pub fn pick_list<'a, T, L, V, Message, Theme, Renderer>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> PickList<'a, T, L, V, Message, Theme, Renderer>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone,
    Theme: pick_list::Catalog + menu::Catalog,
    Renderer: core::text::Renderer,
{
    PickList::new(options, selected, on_selected)
}

/// Creates a new [`MouseArea`] for capturing mouse events.
///
/// This is a sweetened version of [`iced`'s `MouseArea`] where all event
/// handlers receive the cursor position as a [`Point`].
///
/// [`iced`'s `MouseArea`]: https://docs.iced.rs/iced/widget/struct.MouseArea.html
/// [`Point`]: crate::core::Point
pub fn mouse_area<'a, Message, Theme, Renderer>(
    widget: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> MouseArea<'a, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
{
    MouseArea::new(widget)
}
