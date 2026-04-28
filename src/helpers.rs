//! Helper functions to create widgets.

use crate::core;
use crate::core::Element;
use crate::overlay::menu;
use crate::widget::MouseArea;
use crate::widget::button::{self, Button};
use crate::widget::column::{self, Column};
use crate::widget::fit_text::{self, FitText};
use crate::widget::pick_list::{self, PickList};
use crate::widget::row::{self, Row};
use crate::widget::table::{self, Table};
use crate::widget::text_input::{self, TextInput};
use crate::widget::tile_grid::{self, TileGrid};
use crate::widget::toggler::{self, Toggler};
use crate::widget::transition::Transition;

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

/// Creates a new [`Button`] with the given content.
///
/// This is a sweetened version of [`iced`'s `button`] with support for
/// [`on_focus`] and [`on_blur`] messages.
///
/// [`iced`'s `button`]: https://docs.iced.rs/iced/widget/button/index.html
/// [`on_focus`]: Button::on_focus
/// [`on_blur`]: Button::on_blur
pub fn button<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Button<'a, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
    Theme: button::Catalog,
{
    Button::new(content)
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
/// The returned [`PickList`] is disabled until [`on_select`] is called to
/// set the message produced when an option is picked.
///
/// [`iced`'s `pick_list`]: https://docs.iced.rs/iced/widget/pick_list/index.html
/// [`disabled`]: PickList::disabled
/// [`on_select`]: PickList::on_select
pub fn pick_list<'a, T, L, V, Message, Theme, Renderer>(
    selected: Option<V>,
    options: L,
    to_string: impl Fn(&T) -> String + 'a,
) -> PickList<'a, T, L, V, Message, Theme, Renderer>
where
    T: PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    Message: Clone,
    Theme: pick_list::Catalog + menu::Catalog,
    Renderer: core::text::Renderer,
{
    PickList::new(selected, options, to_string)
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

/// Creates a new [`TileGrid`] with the given state and view function.
///
/// The view function is called once for each item, receiving the item's
/// [`ItemId`](crate::widget::tile_grid::ItemId) and a reference to its user data.
///
/// [`TileGrid`]: crate::widget::tile_grid::TileGrid
pub fn tile_grid<'a, T, Message, Theme, Renderer>(
    state: &'a tile_grid::State<T>,
    view: impl Fn(
        tile_grid::ItemId,
        &'a T,
    ) -> tile_grid::Content<'a, Message, Theme, Renderer>,
) -> TileGrid<'a, Message, Theme, Renderer>
where
    Theme: tile_grid::Catalog,
    Renderer: core::Renderer,
{
    TileGrid::new(state, view)
}

/// Creates a new [`Table`] with the given columns and rows.
///
/// Columns can be created using [`table::column`], while rows can be any
/// iterator over some data type `T`.
pub fn table<'a, 'b, T, Message, Theme, Renderer>(
    columns: impl IntoIterator<
        Item = table::Column<'a, 'b, T, Message, Theme, Renderer>,
    >,
    rows: impl IntoIterator<Item = T>,
) -> Table<'a, Message, Theme, Renderer>
where
    T: Clone,
    Message: 'a,
    Theme: table::Catalog,
    Renderer: core::Renderer,
{
    Table::new(columns, rows)
}

/// Creates a new grammar-of-tables [`gt::Table`](crate::widget::gt::Table)
/// from typed columns and rows.
///
/// This is the rich, declarative table for reports and dashboards —
/// title / subtitle / units caption / column labels / stub / body /
/// row groups / summary / grand summary / source notes — with
/// selector-based styling and pluggable number formatters. Reach for
/// the flat [`table`] when you just need a data grid.
pub fn gt<'a, Message, Theme, Renderer>(
    columns: Vec<crate::widget::gt::Column>,
    rows: Vec<Vec<crate::widget::gt::Cell>>,
) -> crate::widget::gt::Table<'a, Message, Theme, Renderer>
where
    Theme: crate::widget::gt::Catalog + iced_widget::text::Catalog + 'a,
    <Theme as iced_widget::text::Catalog>::Class<'a>:
        From<iced_widget::text::StyleFn<'a, Theme>>,
    Renderer: core::text::Renderer<Font = core::Font> + 'a,
{
    crate::widget::gt::Table::new(columns, rows)
}

/// Creates a new [`Toggler`].
///
/// This is a sweetened version of [`iced`'s `toggler`] with a smooth
/// animation when toggling between states.
///
/// [`iced`'s `toggler`]: https://docs.iced.rs/iced/widget/toggler/index.html
pub fn toggler<'a, Message, Theme, Renderer>(
    is_toggled: bool,
) -> Toggler<'a, Message, Theme, Renderer>
where
    Theme: toggler::Catalog,
    Renderer: core::text::Renderer,
{
    Toggler::new(is_toggled)
}

/// Creates a new [`FitText`] from the given content.
///
/// [`FitText`] scales its font size to fit the bounds it is laid out into,
/// up to a configurable ceiling. See the [`fit_text`](mod@crate::widget::fit_text)
/// module docs for the semantics.
pub fn fit_text<'a, Theme, Renderer>(
    content: impl core::text::IntoFragment<'a>,
) -> FitText<'a, Theme, Renderer>
where
    Theme: fit_text::Catalog,
    Renderer: core::text::Renderer,
{
    FitText::new(content)
}

/// Creates a new [`Transition`] showing the given `value`, with `view` as the
/// recipe for materializing an [`Element`] from any value of type `T`.
///
/// Whenever `value` changes between frames (as detected by [`PartialEq`]),
/// the widget animates a slide transition between the previous and new
/// content. The closure must produce an [`Element`] of lifetime `'a` — it
/// cannot borrow from its `&T` argument directly.
pub fn transition<'a, T, Message, Theme, Renderer>(
    value: T,
    view: impl Fn(&T) -> Element<'a, Message, Theme, Renderer> + 'a,
) -> Transition<'a, T, Message, Theme, Renderer>
where
    T: Clone + PartialEq + 'static,
    Renderer: core::Renderer,
{
    Transition::new(value, view)
}
