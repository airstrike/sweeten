//! Sweetened widgets for [`iced`].
//!
//! This module contains enhanced versions of common `iced` widgets. Each widget
//! is a drop-in replacement for its `iced` counterpart, with additional methods
//! for extended functionality.
//!
//! [`iced`]: https://github.com/iced-rs/iced

use iced::advanced::text;
use iced::Element;
use std::borrow::Borrow;

pub mod mouse_area;
pub mod overlay;
pub mod pick_list;
pub mod text_input;

/// Creates a new [`mouse_area::MouseArea`] for capturing mouse events.
///
/// This is a sweetened version of `iced`'s [`MouseArea`] with support for
/// receiving the click position via [`on_press_with`].
///
/// [`MouseArea`]: https://docs.rs/iced/latest/iced/widget/struct.MouseArea.html
/// [`on_press_with`]: mouse_area::MouseArea::on_press_with
pub fn mouse_area<'a, Message, Theme, Renderer>(
    widget: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> mouse_area::MouseArea<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::renderer::Renderer,
{
    mouse_area::MouseArea::new(widget)
}

/// Creates a new [`pick_list::PickList`] for selecting from a list of options.
///
/// This is a sweetened version of `iced`'s [`pick_list`] with support for
/// disabling items in the dropdown.
///
/// [`pick_list`]: https://docs.rs/iced/latest/iced/widget/pick_list/
pub fn pick_list<'a, T, L, V, Message, Theme, Renderer>(
    options: L,
    disabled: Option<impl Fn(&[T]) -> Vec<bool> + 'a>,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> pick_list::PickList<'a, T, L, V, Message, Theme, Renderer>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone,
    Theme: pick_list::Catalog + overlay::menu::Catalog,
    Renderer: text::Renderer,
{
    pick_list::PickList::new(options, disabled, selected, on_selected)
}

/// Creates a new [`text_input::TextInput`].
///
/// This is a sweetened version of `iced`'s [`text_input`] with support for
/// [`on_focus`] and [`on_blur`] messages.
///
/// [`text_input`]: https://docs.rs/iced/latest/iced/widget/text_input/
/// [`on_focus`]: text_input::TextInput::on_focus
/// [`on_blur`]: text_input::TextInput::on_blur
pub fn text_input<'a, Message, Theme, Renderer>(
    placeholder: &str,
    value: &str,
) -> text_input::TextInput<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: text_input::Catalog + 'a,
    Renderer: text::Renderer,
{
    text_input::TextInput::new(placeholder, value)
}
