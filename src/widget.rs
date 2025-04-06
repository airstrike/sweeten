use iced::advanced::text;
use iced::Element;
use local_text_input::LocalTextInput;
use std::borrow::Borrow;

pub mod local_text_input;
pub mod mouse_area;
pub mod overlay;
pub mod pick_list;
pub mod text_input;

/// A container intercepting mouse events.
pub fn mouse_area<'a, Message, Theme, Renderer>(
    widget: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> mouse_area::MouseArea<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::renderer::Renderer,
{
    mouse_area::MouseArea::new(widget)
}

/// Pick lists display a dropdown list of selectable options, some of which
/// may be disabled.
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

/// Creates a new [`TextInput`].
///
/// Text inputs display fields that can be filled with text. This version
/// also allows you to publish messages `.on_focus` and `.on_blur`.
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

/// Creates a new [`LocalTextInput`].
///
/// Text inputs display fields that can be filled with text. This version
/// is an uncontrolled text input that lets you publish messages `.on_submit`
/// and `.on_blur`. It may be useful if the string is only an intermediate
/// state and you don't want to keep it in your app state.
pub fn local_text_input<'a, Message, Theme, Renderer>(
    placeholder: &str,
    initial_value: &str,
) -> LocalTextInput<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: local_text_input::Catalog + 'a,
    Renderer: text::Renderer,
{
    LocalTextInput::new(placeholder, initial_value)
}
