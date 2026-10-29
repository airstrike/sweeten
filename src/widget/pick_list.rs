//! Pick lists display a dropdown list of selectable options.
//!
//! This is a sweetened version of `iced`'s [`pick_list`] with support for:
//!
//! - disabling individual options, inline via [`Entry::disabled`] or
//!   dynamically via [`PickList::disabled`];
//! - titled groups of options via [`group`] and the [`options!`] macro,
//!   spaced with [`Options::spacing`] and optionally separated by a rule
//!   with [`PickList::separator`];
//! - clearing the selection via a [`deselect`] entry — the "None" item of
//!   native menus — paired with [`PickList::on_deselect`];
//! - arbitrary widgets as option content — the view function may return
//!   a `String` or any [`Element`], see [`Content`];
//! - keyboard interaction: arrow keys, Home/End, and typeahead move the
//!   highlighted option, Enter selects it, and Escape closes the menu;
//! - focus: the pick list participates in tab navigation, opens with
//!   Enter, Space, or the arrow keys while focused, and emits
//!   [`PickList::on_focus`] and [`PickList::on_blur`] messages;
//! - a check indicator next to the selected option, a configurable menu
//!   width, and macOS-style selected-item menu anchoring — see
//!   [`PickList::check_indicator`], [`PickList::menu_width`], and
//!   [`PickList::anchor`].
//!
//! [`pick_list`]: https://docs.iced.rs/iced/widget/pick_list/
//! [`options!`]: options
//!
//! # Example
//! ```no_run
//! # pub type Element<'a, Message> = iced::Element<'a, Message>;
//! use sweeten::widget::pick_list;
//!
//! struct State {
//!    favorite: Option<Fruit>,
//! }
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum Fruit {
//!     Apple,
//!     Orange,
//!     Strawberry,
//!     Tomato,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     FruitSelected(Fruit),
//! }
//!
//! fn view(state: &State) -> impl Widget<Message> {
//!     let fruits = [
//!         Fruit::Apple,
//!         Fruit::Orange,
//!         Fruit::Strawberry,
//!         Fruit::Tomato,
//!     ];
//!
//!     // Disable Tomato because it's not a fruit!
//!     pick_list(
//!         state.favorite,
//!         fruits,
//!         Fruit::to_string,
//!     )
//!     .on_select(Message::FruitSelected)
//!     .disabled(|fruit| matches!(fruit, Fruit::Tomato))
//!     .placeholder("Select your favorite fruit...")
//! }
//!
//! impl std::fmt::Display for Fruit {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         f.write_str(match self {
//!             Self::Apple => "Apple",
//!             Self::Orange => "Orange",
//!             Self::Strawberry => "Strawberry",
//!             Self::Tomato => "Tomato",
//!         })
//!     }
//! }
//! ```
//!
//! Options can be arranged in titled [`Group`]s with [`group`] and the
//! [`options!`] macro, and disabled inline with [`option`]. A literal
//! `None` reads as a [`deselect`] entry:
//!
//! ```no_run
//! # pub type Element<'a, Message> = iced::Element<'a, Message>;
//! use sweeten::pick_list;
//! use sweeten::widget::pick_list::{group, option, options};
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum Food {
//!     Apple,
//!     Banana,
//!     Carrot,
//! }
//! # #[derive(Debug, Clone)]
//! # enum Message { Picked(Food), Cleared }
//! # struct State { pick: Option<Food> }
//!
//! fn view(state: &State) -> impl Widget<Message> {
//!     pick_list(
//!         state.pick,
//!         options![
//!             None,
//!             group("Fruits", [option(Food::Apple), option(Food::Banana)]),
//!             group("Vegetables", [option(Food::Carrot).disabled()]),
//!         ],
//!         |food| format!("{food:?}"),
//!     )
//!     .on_select(Message::Picked)
//!     .on_deselect(Message::Cleared)
//!     .separator(true)
//! }
//! ```
use crate::core::alignment;
use crate::core::border;
use crate::core::keyboard;
use crate::core::keyboard::key;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::text::paragraph;
use crate::core::text::{self, Text};
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::Meta;
use crate::core::widget::Operation;
use crate::core::widget::operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Background, Border, Color, Element, Event, Font, Layout, Length, Padding,
    Pixels, Point, Rectangle, Shell, Size, Theme, Vector, Widget,
};
use crate::overlay::menu::{self, Menu};

use std::borrow::Borrow;
use std::convert::Infallible;
use std::f32;

/// A widget for selecting a single value from a list of options.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use sweeten::widget::pick_list;
///
/// struct State {
///    favorite: Option<Fruit>,
/// }
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum Fruit {
///     Apple,
///     Orange,
///     Strawberry,
///     Tomato,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     FruitSelected(Fruit),
/// }
///
/// fn view(state: &State) -> impl Widget<Message> {
///     let fruits = [
///         Fruit::Apple,
///         Fruit::Orange,
///         Fruit::Strawberry,
///         Fruit::Tomato,
///     ];
///
///     pick_list(
///         state.favorite,
///         fruits,
///         Fruit::to_string,
///     )
///     .on_select(Message::FruitSelected)
///     .placeholder("Select your favorite fruit...")
/// }
///
/// fn update(state: &mut State, message: Message) {
///     match message {
///         Message::FruitSelected(fruit) => {
///             state.favorite = Some(fruit);
///         }
///     }
/// }
///
/// impl std::fmt::Display for Fruit {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         f.write_str(match self {
///             Self::Apple => "Apple",
///             Self::Orange => "Orange",
///             Self::Strawberry => "Strawberry",
///             Self::Tomato => "Tomato",
///         })
///     }
/// }
/// ```
#[allow(clippy::type_complexity)]
pub struct PickList<
    'a,
    T,
    V,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    T: PartialEq + Clone,
    V: Borrow<T> + 'a,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    id: Option<widget::Id>,
    options: Options<'a, T, Theme, Renderer>,
    view: Box<dyn Fn(&T) -> Content<'a, Theme, Renderer> + 'a>,
    trigger: Option<Content<'a, Theme, Renderer>>,
    on_select: Option<Box<dyn Fn(T) -> Message + 'a>>,
    on_deselect: Option<Message>,
    on_open: Option<Message>,
    on_close: Option<Message>,
    on_focus: Option<Message>,
    on_blur: Option<Message>,
    on_option_hovered: Option<Box<dyn Fn(T) -> Message + 'a>>,
    disabled: Option<Box<dyn Fn(&T) -> bool + 'a>>,
    typeahead: Option<Box<dyn Fn(&T) -> String + 'a>>,
    placeholder: Option<Content<'a, Theme, Renderer>>,
    selected: Option<V>,
    width: Length,
    padding: Padding,
    radius: Option<border::Radius>,
    text_size: Option<Pixels>,
    line_height: Option<text::LineHeight>,
    shaping: text::Shaping,
    ellipsis: text::Ellipsis,
    font: Option<Font>,
    handle: Handle<Font>,
    class: <Theme as Catalog>::Class<'a>,
    menu_class: <Theme as menu::Catalog>::Class<'a>,
    last_status: Option<Status>,
    menu_height: Length,
    menu_width: Option<Length>,
    menu_padding: Padding,
    anchor: Anchor,
    separator: bool,
    check_indicator: bool,
}

impl<'a, T, V, Message, Theme, Renderer>
    PickList<'a, T, V, Message, Theme, Renderer>
where
    T: PartialEq + Clone,
    V: Borrow<T> + 'a,
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// Creates a new [`PickList`] with the given list of options, the current
    /// selected value, and a function producing the [`Content`] displayed
    /// for each option.
    ///
    /// The options can be a plain list — a `Vec<T>`, a slice, or an array —
    /// or richer [`Options`] built with [`group`], [`deselect`], and
    /// [`option`], most comfortably through the [`options!`] macro.
    ///
    /// [`options!`]: options
    ///
    /// The view function may return a `String` (or anything else that
    /// converts into [`Content`], like an [`Element`]) and is used both for
    /// the options in the menu and for the selected value.
    #[allow(clippy::type_complexity)]
    pub fn new<W>(
        selected: Option<V>,
        options: impl Into<Options<'a, T, Theme, Renderer>>,
        view: impl Fn(&T) -> W + 'a,
    ) -> Self
    where
        W: Into<Content<'a, Theme, Renderer>>,
    {
        let view: Box<dyn Fn(&T) -> Content<'a, Theme, Renderer> + 'a> =
            Box::new(move |value| view(value).into());

        let trigger = selected.as_ref().map(|selected| view(selected.borrow()));

        Self {
            id: Some(widget::Id::unique()),
            view,
            trigger,
            on_select: None,
            on_deselect: None,
            on_open: None,
            on_close: None,
            on_focus: None,
            on_blur: None,
            on_option_hovered: None,
            options: options.into(),
            disabled: None,
            typeahead: None,
            placeholder: None,
            selected,
            width: Length::Shrink,
            padding: crate::button::DEFAULT_PADDING,
            radius: None,
            text_size: None,
            line_height: None,
            shaping: text::Shaping::default(),
            ellipsis: text::Ellipsis::End,
            font: None,
            handle: Handle::default(),
            class: <Theme as Catalog>::default(),
            menu_class: <Theme as Catalog>::default_menu(),
            last_status: None,
            menu_height: Length::Shrink,
            menu_width: None,
            menu_padding: Padding::new(4.0),
            anchor: Anchor::default(),
            separator: false,
            check_indicator: true,
        }
    }

    /// Sets the placeholder of the [`PickList`], displayed when nothing
    /// is selected.
    ///
    /// Like the option contents, it can be plain text — drawn muted — or
    /// any [`Element`], displayed as-is.
    pub fn placeholder(
        mut self,
        placeholder: impl Into<Content<'a, Theme, Renderer>>,
    ) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Sets a function that determines which options are disabled.
    ///
    /// The function receives each option and returns `true` if it is
    /// disabled. Options can also be disabled inline with
    /// [`Entry::disabled`]; an option is disabled if either says so.
    pub fn disabled(mut self, disabled: impl Fn(&T) -> bool + 'a) -> Self {
        self.disabled = Some(Box::new(disabled));
        self
    }

    /// Sets the text used to match keyboard typeahead input against an
    /// option whose [`Content`] is an element.
    ///
    /// Options displayed as [`Content::Text`] already match typeahead
    /// input against their text; this is only needed for options rendered
    /// as arbitrary widgets.
    pub fn typeahead(mut self, typeahead: impl Fn(&T) -> String + 'a) -> Self {
        self.typeahead = Some(Box::new(typeahead));
        self
    }

    /// Sets the width of the [`PickList`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Menu`].
    pub fn menu_height(mut self, menu_height: impl Into<Length>) -> Self {
        self.menu_height = menu_height.into();
        self
    }

    /// Sets the width of the [`Menu`].
    ///
    /// By default — and with [`Length::Shrink`] — the menu fits its widest
    /// entry, but never gets narrower than the [`PickList`] itself.
    pub fn menu_width(mut self, menu_width: impl Into<Length>) -> Self {
        self.menu_width = Some(menu_width.into());
        self
    }

    /// Sets the inner [`Padding`] of the [`Menu`], inset between its
    /// border and its contents.
    ///
    /// Defaults to `4` on every side.
    pub fn menu_padding(mut self, padding: impl Into<Padding>) -> Self {
        self.menu_padding = padding.into();
        self
    }

    /// Sets the [`Anchor`] of the [`Menu`].
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets whether a horizontal rule separates the [`Group`]s of the
    /// [`Menu`], drawn centered in the [`Options::spacing`] gap.
    ///
    /// Disabled by default.
    pub fn separator(mut self, separator: bool) -> Self {
        self.separator = separator;
        self
    }

    /// Sets whether the [`Menu`] displays a check indicator next to the
    /// selected option.
    ///
    /// Enabled by default.
    pub fn check_indicator(mut self, check_indicator: bool) -> Self {
        self.check_indicator = check_indicator;
        self
    }

    /// Sets the [`Padding`] of the [`PickList`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the border radius of the whole [`PickList`]: its border, the
    /// menu border, and the menu highlights, which derive theirs from it
    /// reduced by `2` — the radius step between a menu and its items in
    /// common design systems.
    ///
    /// This overrides the radius of the [`Style`] and of the menu
    /// [`Style`](menu::Style).
    pub fn radius(mut self, radius: impl Into<border::Radius>) -> Self {
        self.radius = Some(radius.into());
        self
    }

    /// Sets the text size of the [`PickList`].
    pub fn text_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`PickList`].
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = Some(line_height.into());
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`PickList`].
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the [`text::Ellipsis`] strategy of the [`PickList`].
    pub fn ellipsis(mut self, ellipsis: text::Ellipsis) -> Self {
        self.ellipsis = ellipsis;
        self
    }

    /// Sets the font of the [`PickList`].
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the [`Handle`] of the [`PickList`].
    pub fn handle(mut self, handle: Handle<Font>) -> Self {
        self.handle = handle;
        self
    }

    /// Sets the message that will be produced when the [`PickList`]
    /// selected value changes.
    pub fn on_select(mut self, on_select: impl Fn(T) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// Sets the message that will be produced when a [`deselect`] entry of
    /// the [`PickList`] clears the selection.
    ///
    /// Without this handler, [`deselect`] entries are disabled.
    pub fn on_deselect(mut self, on_deselect: Message) -> Self {
        self.on_deselect = Some(on_deselect);
        self
    }

    /// Sets the message that will be produced when the [`PickList`] is opened.
    pub fn on_open(mut self, on_open: Message) -> Self {
        self.on_open = Some(on_open);
        self
    }

    /// Sets the message that will be produced when the [`PickList`] is closed.
    pub fn on_close(mut self, on_close: Message) -> Self {
        self.on_close = Some(on_close);
        self
    }

    /// Sets the message that will be produced when the [`PickList`] gains
    /// keyboard focus.
    pub fn on_focus(mut self, on_focus: Message) -> Self {
        self.on_focus = Some(on_focus);
        self
    }

    /// Sets the message that will be produced when the [`PickList`] loses
    /// keyboard focus.
    pub fn on_blur(mut self, on_blur: Message) -> Self {
        self.on_blur = Some(on_blur);
        self
    }

    /// Sets the [`widget::Id`] of the [`PickList`].
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the message that will be produced when an option of the
    /// [`PickList`] is highlighted, either by hovering it with the pointer
    /// or through keyboard navigation.
    pub fn on_option_hovered(
        mut self,
        on_option_hovered: impl Fn(T) -> Message + 'a,
    ) -> Self {
        self.on_option_hovered = Some(Box::new(on_option_hovered));
        self
    }

    /// Sets the style of the [`PickList`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        <Theme as Catalog>::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style of the [`Menu`].
    #[must_use]
    pub fn menu_style(
        mut self,
        style: impl Fn(&Theme) -> menu::Style + 'a,
    ) -> Self
    where
        <Theme as menu::Catalog>::Class<'a>: From<menu::StyleFn<'a, Theme>>,
    {
        self.menu_class = (Box::new(style) as menu::StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`PickList`].
    #[must_use]
    pub fn class(
        mut self,
        class: impl Into<<Theme as Catalog>::Class<'a>>,
    ) -> Self {
        self.class = class.into();
        self
    }

    /// Sets the style class of the [`Menu`].
    #[must_use]
    pub fn menu_class(
        mut self,
        class: impl Into<<Theme as menu::Catalog>::Class<'a>>,
    ) -> Self {
        self.menu_class = class.into();
        self
    }
}

impl<'a, T, V, Message, Theme, Renderer> Meta
    for PickList<'a, T, V, Message, Theme, Renderer>
where
    T: PartialEq + Clone,
    V: Borrow<T> + 'a,
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
}

impl<'a, T, V, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PickList<'a, T, V, Message, Theme, Renderer>
where
    T: Clone + PartialEq + 'a,
    V: Borrow<T>,
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::new())
    }

    fn diff(&mut self, tree: &mut Tree) {
        let content = match (&mut self.trigger, &mut self.placeholder) {
            (Some(trigger), _) => Some(trigger),
            (None, placeholder) => placeholder.as_mut(),
        };

        if let Some(Content::Element(element)) = content {
            tree.diff_children(std::slice::from_mut(element));
        } else {
            tree.children.clear();
        }
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        let font = self.font.unwrap_or_else(|| renderer.font());
        let text_size = self.text_size.unwrap_or_else(|| renderer.text_size());
        let line_height =
            self.line_height.unwrap_or_else(|| renderer.line_height());

        let option_text = Text {
            content: "",
            bounds: Size::new(
                f32::INFINITY,
                line_height.to_absolute(text_size).into(),
            ),
            size: text_size,
            line_height,
            font,
            align_x: text::Alignment::Default,
            align_y: alignment::Vertical::Center,
            shaping: self.shaping,
            wrapping: text::Wrapping::None,
            ellipsis: self.ellipsis,
            hint_factor: renderer.hint_factor(),
        };

        if let Some(Content::Text(placeholder)) = &self.placeholder {
            let _ = state.placeholder.update(Text {
                content: placeholder,
                ..option_text
            });
        }

        let max_width = if matches!(self.width, Length::Shrink | Length::Fit) {
            let count = self.options.slots().count();
            state.options.resize_with(count, Default::default);

            let labels = self
                .options
                .groups()
                .iter()
                .flat_map(|group| group.entries.iter())
                .map(|entry| match entry {
                    Entry::Option { value, .. } => match (self.view)(value) {
                        Content::Text(label) => label,
                        Content::Element(_) => String::new(),
                    },
                    Entry::Deselect(Content::Text(label)) => label.clone(),
                    Entry::Deselect(Content::Element(_)) => String::new(),
                });

            for (label, paragraph) in labels.zip(state.options.iter_mut()) {
                let _ = paragraph.update(Text {
                    content: &label,
                    ..option_text
                });
            }

            let labels_width =
                state.options.iter().fold(0.0, |width, paragraph| {
                    f32::max(width, paragraph.min_width())
                });

            labels_width.max(match &self.placeholder {
                Some(Content::Text(_)) => state.placeholder.min_width(),
                _ => 0.0,
            })
        } else {
            0.0
        };

        // the widest label must also fit a row of the menu: rows keep
        // their text aligned with the pick list's, and add the check
        // indicator gutter on the right
        let allowance = {
            let gutter = if self.check_indicator {
                text_size.0 * 1.5
            } else {
                0.0
            };
            let row_inset = (self.padding.left - self.menu_padding.left)
                .max(0.0)
                + (self.padding.right - self.menu_padding.right).max(0.0);

            text_size.0.max(
                row_inset + gutter + self.menu_padding.x()
                    - self.padding.left
                    - self.padding.x(),
            )
        };

        let content = match (&mut self.trigger, &mut self.placeholder) {
            (Some(trigger), _) => Some(trigger),
            (None, placeholder) => placeholder.as_mut(),
        };

        if let Some(Content::Element(element)) = content {
            let content_limits = limits.width(self.width).shrink(self.padding);
            let child_limits = layout::Limits::new(
                Size::ZERO,
                Size::new(
                    (content_limits.bounds().width
                        - allowance
                        - self.padding.left)
                        .max(0.0),
                    content_limits.bounds().height,
                ),
            );

            element.layout(&mut tree.children[0], renderer, &child_limits);
            let child_size = tree.children[0].size;

            let line = f32::from(line_height.to_absolute(text_size));
            let content_height = child_size.height.max(line);

            let intrinsic = Size::new(
                max_width.max(child_size.width) + allowance + self.padding.left,
                content_height,
            );

            let size = limits
                .width(self.width)
                .shrink(self.padding)
                .resolve(self.width, Length::Shrink, intrinsic)
                .expand(self.padding);

            tree.size = size;

            let inner_height = size.height - self.padding.y();
            tree.children[0].translation = Vector::new(
                self.padding.left,
                self.padding.top + (inner_height - child_size.height) / 2.0,
            );
        } else {
            let size = {
                let intrinsic = Size::new(
                    max_width + allowance + self.padding.left,
                    f32::from(line_height.to_absolute(text_size)),
                );

                limits
                    .width(self.width)
                    .shrink(self.padding)
                    .resolve(self.width, Length::Shrink, intrinsic)
                    .expand(self.padding)
            };

            tree.size = size;
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        _viewport: &Rectangle,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.on_select.is_some() {
            let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

            operation.focusable(self.id.as_ref(), layout.bounds(), state);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        // Detect focus changes from operations (e.g., focus_next). Losing
        // focus also closes the menu, so it cannot outlive the focus that
        // opened it.
        if state.is_focused != state.was_focused {
            if state.is_focused {
                if let Some(on_focus) = &self.on_focus {
                    shell.publish(on_focus.clone());
                }
            } else {
                if let Some(on_blur) = &self.on_blur {
                    shell.publish(on_blur.clone());
                }

                if state.is_open {
                    state.is_open = false;

                    if let Some(on_close) = &self.on_close {
                        shell.publish(on_close.clone());
                    }
                }
            }

            state.was_focused = state.is_focused;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let is_over = cursor.is_over(layout.bounds());

                if is_over && self.on_select.is_some() && !state.is_focused {
                    state.is_focused = true;
                    state.was_focused = true;

                    if let Some(on_focus) = &self.on_focus {
                        shell.publish(on_focus.clone());
                    }
                } else if !is_over && state.is_focused {
                    state.is_focused = false;
                    state.was_focused = false;

                    if let Some(on_blur) = &self.on_blur {
                        shell.publish(on_blur.clone());
                    }
                }

                if state.is_open {
                    // Event wasn't processed by overlay, so cursor was clicked either outside its
                    // bounds or on the drop-down, either way we close the overlay.
                    state.is_open = false;

                    if let Some(on_close) = &self.on_close {
                        shell.publish(on_close.clone());
                    }

                    shell.capture_event();
                } else if is_over {
                    let selected = self.selected.as_ref().map(Borrow::borrow);

                    state.is_open = true;
                    state.hovered_option =
                        self.options.slots().position(|slot| slot == selected);

                    if let Some(on_open) = &self.on_open {
                        shell.publish(on_open.clone());
                    }

                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { y, .. },
            }) => {
                let Some(on_select) = &self.on_select else {
                    return;
                };

                if state.keyboard_modifiers.command()
                    && cursor.is_over(layout.bounds())
                    && !state.is_open
                {
                    fn find_next<'a, T: PartialEq>(
                        selected: &'a T,
                        mut options: impl Iterator<Item = &'a T>,
                    ) -> Option<&'a T> {
                        let _ = options.find(|&option| option == selected);

                        options.next()
                    }

                    let selected = self.selected.as_ref().map(Borrow::borrow);

                    let next_option = if *y < 0.0 {
                        if let Some(selected) = selected {
                            find_next(selected, self.options.values())
                        } else {
                            self.options.values().next()
                        }
                    } else if *y > 0.0 {
                        if let Some(selected) = selected {
                            find_next(selected, self.options.values().rev())
                        } else {
                            self.options.values().next_back()
                        }
                    } else {
                        None
                    };

                    if let Some(next_option) = next_option {
                        shell.publish(on_select(next_option.clone()));
                    }

                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) if state.is_open => {
                state.is_open = false;

                if let Some(on_close) = &self.on_close {
                    shell.publish(on_close.clone());
                }

                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key:
                    keyboard::Key::Named(
                        key::Named::Enter
                        | key::Named::Space
                        | key::Named::ArrowDown
                        | key::Named::ArrowUp,
                    ),
                ..
            }) if state.is_focused
                && !state.is_open
                && self.on_select.is_some() =>
            {
                let selected = self.selected.as_ref().map(Borrow::borrow);

                state.is_open = true;
                state.hovered_option =
                    self.options.slots().position(|slot| slot == selected);

                if let Some(on_open) = &self.on_open {
                    shell.publish(on_open.clone());
                }

                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.keyboard_modifiers = *modifiers;
            }
            _ => {}
        };

        let status = {
            let is_hovered = cursor.is_over(layout.bounds());

            if self.on_select.is_none() {
                Status::Disabled
            } else if state.is_open {
                Status::Opened { is_hovered }
            } else if state.is_focused {
                Status::Focused { is_hovered }
            } else if is_hovered {
                Status::Hovered
            } else {
                Status::Active
            }
        };

        if let Event::Window(window::Event::RedrawRequested(_now)) = event {
            self.last_status = Some(status);
        } else if self
            .last_status
            .is_some_and(|last_status| last_status != status)
        {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        if is_mouse_over {
            if self.on_select.is_some() {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::Idle
            }
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let font = self.font.unwrap_or_else(|| renderer.font());
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        let bounds = layout.bounds();

        let style = Catalog::style(
            theme,
            &self.class,
            self.last_status.unwrap_or(Status::Active),
        );

        let border = Border {
            radius: self.radius.unwrap_or(style.border.radius),
            ..style.border
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border,
                ..renderer::Quad::default()
            },
            style.background,
        );

        let handle = match &self.handle {
            Handle::Arrow { size } => Some((
                Renderer::ICON_FONT,
                Renderer::ARROW_DOWN_ICON,
                *size,
                None,
                text::Shaping::Basic,
            )),
            Handle::Static(Icon {
                font,
                code_point,
                size,
                line_height,
                shaping,
            }) => Some((*font, *code_point, *size, *line_height, *shaping)),
            Handle::Dynamic { open, closed } => {
                if state.is_open {
                    Some((
                        open.font,
                        open.code_point,
                        open.size,
                        open.line_height,
                        open.shaping,
                    ))
                } else {
                    Some((
                        closed.font,
                        closed.code_point,
                        closed.size,
                        closed.line_height,
                        closed.shaping,
                    ))
                }
            }
            Handle::None => None,
        };

        if let Some((font, code_point, size, line_height, shaping)) = handle {
            let size = size.unwrap_or_else(|| renderer.text_size());
            let line_height =
                line_height.unwrap_or_else(|| renderer.line_height());

            renderer.fill_text(
                Text {
                    content: code_point.to_string(),
                    size,
                    line_height,
                    font,
                    bounds: Size::new(
                        bounds.width,
                        f32::from(line_height.to_absolute(size)),
                    ),
                    align_x: text::Alignment::Right,
                    align_y: alignment::Vertical::Center,
                    shaping,
                    wrapping: text::Wrapping::None,
                    ellipsis: text::Ellipsis::None,
                    hint_factor: None,
                },
                Point::new(
                    bounds.x + bounds.width - self.padding.right,
                    bounds.center_y(),
                ),
                style.handle_color,
                *viewport,
            );
        }

        let content = self.trigger.as_ref().or(self.placeholder.as_ref());
        let is_selected = self.trigger.is_some();

        match content {
            Some(Content::Element(element)) => {
                if let Some((child_layout, child_tree)) =
                    layout.iter(&tree.children).next()
                {
                    element.draw(
                        child_tree,
                        renderer,
                        theme,
                        &renderer::Style {
                            text_color: if is_selected {
                                style.text_color
                            } else {
                                style.placeholder_color
                            },
                        },
                        child_layout,
                        cursor,
                        viewport,
                    );
                }
            }
            Some(Content::Text(label)) => {
                let text_size =
                    self.text_size.unwrap_or_else(|| renderer.text_size());
                let line_height =
                    self.line_height.unwrap_or_else(|| renderer.line_height());

                renderer.fill_text(
                    Text {
                        content: label.clone(),
                        size: text_size,
                        line_height: line_height,
                        font,
                        bounds: Size::new(
                            bounds.width - self.padding.x(),
                            f32::from(line_height.to_absolute(text_size)),
                        ),
                        align_x: text::Alignment::Default,
                        align_y: alignment::Vertical::Center,
                        shaping: self.shaping,
                        wrapping: text::Wrapping::None,
                        ellipsis: self.ellipsis,
                        hint_factor: renderer.hint_factor(),
                    },
                    Point::new(bounds.x + self.padding.left, bounds.center_y()),
                    if is_selected {
                        style.text_color
                    } else {
                        style.placeholder_color
                    },
                    *viewport,
                );
            }
            None => {}
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout,
        renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        let Some(on_select) = &self.on_select else {
            return Vec::new();
        };

        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let font = self.font.unwrap_or_else(|| renderer.font());
        let line_height =
            self.line_height.unwrap_or_else(|| renderer.line_height());

        if state.is_open {
            let bounds = layout.bounds();

            let has_deselect = self.on_deselect.is_some();

            let disabled = Some(
                self.options
                    .slots()
                    .map(|slot| match slot {
                        Some(value) => self
                            .disabled
                            .as_ref()
                            .is_some_and(|is_disabled| is_disabled(value)),
                        None => !has_deselect,
                    })
                    .collect(),
            );

            let selected = self.selected.as_ref().map(Borrow::borrow);
            let selected_index =
                self.options.slots().position(|slot| slot == selected);

            let on_deselect = &self.on_deselect;

            let mut menu = Menu::new(
                &mut state.menu,
                &mut self.options,
                &mut state.hovered_option,
                &*self.view,
                |selection| {
                    state.is_open = false;

                    match selection {
                        Some(option) => (on_select)(option),
                        None => on_deselect.clone().expect(
                            "on_deselect handler must exist: deselect \
                             entries are disabled without one",
                        ),
                    }
                },
                disabled,
                self.typeahead.as_deref(),
                self.on_option_hovered.as_deref(),
                &self.menu_class,
            )
            .width(bounds.width)
            .height(self.menu_height)
            .selected(selected_index)
            .check_indicator(self.check_indicator)
            .anchor(self.anchor)
            .separator(self.separator)
            .menu_padding(self.menu_padding)
            .padding(self.padding)
            .font(font)
            .line_height(line_height)
            .ellipsis(self.ellipsis)
            .shaping(self.shaping);

            if let Some(text_size) = self.text_size {
                menu = menu.text_size(text_size);
            }

            if let Some(menu_width) = self.menu_width {
                menu = menu.menu_width(menu_width);
            }

            if let Some(radius) = self.radius {
                menu = menu.target_radius(radius);
            }

            vec![menu.overlay(
                renderer,
                layout.position() + translation,
                window,
                bounds.height,
            )]
        } else {
            Vec::new()
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __sweeten_pick_list_options {
    () => (
        $crate::widget::pick_list::Options::default()
    );
    ($($group:expr),+ $(,)?) => (
        $crate::widget::pick_list::Options::with_groups([
            $($crate::widget::pick_list::Group::from($group)),+
        ])
    );
}

/// Creates [`Options`] from the given groups.
///
/// Each item converts into a [`Group`]: [`group`] and [`deselect`]
/// calls, plain lists of values, or an `Option<T>` — `None` reads as a
/// deselect entry labeled `"None"`.
#[doc(inline)]
pub use crate::__sweeten_pick_list_options as options;

/// The menu of a [`PickList`]: a list of [`Group`]s of options.
///
/// Plain lists of values convert into [`Options`] directly — a single,
/// untitled group — so most call sites can simply pass a `Vec<T>`, a
/// slice, or an array. Richer menus compose out of [`group`],
/// [`deselect`], and [`option`], most comfortably with the
/// [`options!`] macro:
///
/// ```no_run
/// use sweeten::widget::pick_list::{group, option, options};
///
/// # fn view() -> sweeten::widget::pick_list::Options<'static, &'static str> {
/// options![
///     group("Fruits", [option("Apple"), option("Banana")]),
///     group("Vegetables", [option("Carrot").disabled()]),
///     ["Kiwi", "Mango"], // an untitled group
/// ]
/// # }
/// ```
///
/// [`options!`]: options
pub struct Options<'a, T, Theme = crate::Theme, Renderer = crate::Renderer> {
    pub(crate) groups: Vec<Group<'a, T, Theme, Renderer>>,
    pub(crate) spacing: Pixels,
}

impl<'a, T, Theme, Renderer> Options<'a, T, Theme, Renderer> {
    /// Creates [`Options`] from the given list of [`Group`]s.
    pub fn with_groups(
        groups: impl IntoIterator<Item = impl Into<Group<'a, T, Theme, Renderer>>>,
    ) -> Self {
        Self {
            groups: groups.into_iter().map(Into::into).collect(),
            spacing: Pixels(8.0),
        }
    }

    /// Sets the vertical spacing between the [`Group`]s of the [`Options`].
    ///
    /// Defaults to `8`. When [`PickList::separator`] is enabled, the
    /// separating rule is drawn centered in this gap.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into();
        self
    }

    /// Returns the groups of the [`Options`].
    pub fn groups(&self) -> &[Group<'a, T, Theme, Renderer>] {
        &self.groups
    }

    /// Returns an iterator over the selectable values of the [`Options`],
    /// in order, skipping titles and deselect entries.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.groups
            .iter()
            .flat_map(|group| group.entries.iter())
            .filter_map(Entry::value)
    }

    /// Iterates over the selectable slots — options and deselect entries —
    /// in the order the menu indexes them.
    fn slots(&self) -> impl DoubleEndedIterator<Item = Option<&T>> {
        self.groups
            .iter()
            .flat_map(|group| group.entries.iter())
            .map(|entry| match entry {
                Entry::Option { value, .. } => Some(value),
                Entry::Deselect(_) => None,
            })
    }
}

impl<T, Theme, Renderer> Default for Options<'_, T, Theme, Renderer> {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            spacing: Pixels(8.0),
        }
    }
}

impl<'a, T, Theme, Renderer> From<Group<'a, T, Theme, Renderer>>
    for Options<'a, T, Theme, Renderer>
{
    fn from(group: Group<'a, T, Theme, Renderer>) -> Self {
        Self::with_groups([group])
    }
}

impl<'a, T, Theme, Renderer> From<Vec<Group<'a, T, Theme, Renderer>>>
    for Options<'a, T, Theme, Renderer>
{
    fn from(groups: Vec<Group<'a, T, Theme, Renderer>>) -> Self {
        Self::with_groups(groups)
    }
}

impl<'a, T, Theme, Renderer, const N: usize>
    From<[Group<'a, T, Theme, Renderer>; N]>
    for Options<'a, T, Theme, Renderer>
{
    fn from(groups: [Group<'a, T, Theme, Renderer>; N]) -> Self {
        Self::with_groups(groups)
    }
}

impl<'a, T, Theme, Renderer> From<Vec<T>> for Options<'a, T, Theme, Renderer> {
    fn from(values: Vec<T>) -> Self {
        Group::from(values).into()
    }
}

impl<'a, T: Clone, Theme, Renderer> From<&[T]>
    for Options<'a, T, Theme, Renderer>
{
    fn from(values: &[T]) -> Self {
        Group::from(values).into()
    }
}

impl<'a, T: Clone, Theme, Renderer> From<&Vec<T>>
    for Options<'a, T, Theme, Renderer>
{
    fn from(values: &Vec<T>) -> Self {
        Group::from(values).into()
    }
}

impl<'a, T, Theme, Renderer, const N: usize> From<[T; N]>
    for Options<'a, T, Theme, Renderer>
{
    fn from(values: [T; N]) -> Self {
        Group::from(values).into()
    }
}

impl<'a, T: Clone, Theme, Renderer, const N: usize> From<&[T; N]>
    for Options<'a, T, Theme, Renderer>
{
    fn from(values: &[T; N]) -> Self {
        Group::from(values).into()
    }
}

/// A run of entries in the menu of a [`PickList`], with an optional
/// title.
///
/// Groups are created with [`group`] — or [`Group::new`] for an untitled
/// one — and plain lists of values convert into untitled groups.
pub struct Group<'a, T, Theme = crate::Theme, Renderer = crate::Renderer> {
    pub(crate) title: Option<Content<'a, Theme, Renderer>>,
    pub(crate) entries: Vec<Entry<'a, T, Theme, Renderer>>,
}

impl<'a, T, Theme, Renderer> Group<'a, T, Theme, Renderer> {
    /// Creates an untitled [`Group`] with the given entries.
    pub fn new(
        entries: impl IntoIterator<Item = impl Into<Entry<'a, T, Theme, Renderer>>>,
    ) -> Self {
        Self {
            title: None,
            entries: entries.into_iter().map(Into::into).collect(),
        }
    }

    /// Sets the title of the [`Group`].
    ///
    /// Plain text is drawn slightly smaller and muted; an [`Element`] is
    /// displayed as-is.
    pub fn title(
        mut self,
        title: impl Into<Content<'a, Theme, Renderer>>,
    ) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl<'a, T, Theme, Renderer> From<Vec<T>> for Group<'a, T, Theme, Renderer> {
    fn from(values: Vec<T>) -> Self {
        Self::new(values)
    }
}

impl<'a, T: Clone, Theme, Renderer> From<&[T]>
    for Group<'a, T, Theme, Renderer>
{
    fn from(values: &[T]) -> Self {
        Self::new(values.to_vec())
    }
}

impl<'a, T: Clone, Theme, Renderer> From<&Vec<T>>
    for Group<'a, T, Theme, Renderer>
{
    fn from(values: &Vec<T>) -> Self {
        Self::new(values.clone())
    }
}

impl<'a, T, Theme, Renderer, const N: usize> From<[T; N]>
    for Group<'a, T, Theme, Renderer>
{
    fn from(values: [T; N]) -> Self {
        Self::new(values)
    }
}

impl<'a, T: Clone, Theme, Renderer, const N: usize> From<&[T; N]>
    for Group<'a, T, Theme, Renderer>
{
    fn from(values: &[T; N]) -> Self {
        Self::new(values.to_vec())
    }
}

impl<'a, T, Theme, Renderer> From<Vec<Entry<'a, T, Theme, Renderer>>>
    for Group<'a, T, Theme, Renderer>
{
    fn from(entries: Vec<Entry<'a, T, Theme, Renderer>>) -> Self {
        Self::new(entries)
    }
}

impl<'a, T, Theme, Renderer, const N: usize>
    From<[Entry<'a, T, Theme, Renderer>; N]> for Group<'a, T, Theme, Renderer>
{
    fn from(entries: [Entry<'a, T, Theme, Renderer>; N]) -> Self {
        Self::new(entries)
    }
}

/// `None` reads as a [`deselect`] entry labeled `"None"`, and `Some(value)`
/// as a single untitled option, so `Option<T>` slots directly into
/// [`options!`]:
///
/// ```no_run
/// use sweeten::widget::pick_list::{group, options};
///
/// # fn view() -> sweeten::widget::pick_list::Options<'static, &'static str> {
/// options![
///     None,
///     group("Fruits", ["Apple", "Banana"]),
/// ]
/// # }
/// ```
///
/// Use [`deselect`] to label the entry with something other than `"None"`.
///
/// [`options!`]: options
impl<'a, T, Theme, Renderer> From<Option<T>> for Group<'a, T, Theme, Renderer> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::new([value]),
            None => Self {
                title: None,
                entries: vec![Entry::Deselect(Content::Text(
                    "None".to_owned(),
                ))],
            },
        }
    }
}

/// An entry in a [`Group`] of a [`PickList`] menu.
pub enum Entry<'a, T, Theme = crate::Theme, Renderer = crate::Renderer> {
    /// A selectable option.
    Option {
        /// The value produced when this option is selected.
        value: T,
        /// Whether the option cannot be selected.
        disabled: bool,
    },
    /// A selectable entry that clears the current selection — the "None"
    /// item of native menus.
    ///
    /// Selecting it produces the [`PickList::on_deselect`] message, and
    /// it displays the check indicator while nothing is selected. Without
    /// an [`PickList::on_deselect`] handler, the entry is disabled.
    Deselect(Content<'a, Theme, Renderer>),
}

impl<'a, T, Theme, Renderer> Entry<'a, T, Theme, Renderer> {
    /// Disables the [`Entry`], if it is an option.
    pub fn disabled(mut self) -> Self {
        if let Self::Option { disabled, .. } = &mut self {
            *disabled = true;
        }

        self
    }

    /// Returns the value of the [`Entry`], if it is an option.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Option { value, .. } => Some(value),
            Self::Deselect(_) => None,
        }
    }
}

impl<'a, T, Theme, Renderer> From<T> for Entry<'a, T, Theme, Renderer> {
    fn from(value: T) -> Self {
        Self::Option {
            value,
            disabled: false,
        }
    }
}

/// Creates a [`Group`] with the given title and entries.
///
/// Plain text titles are drawn slightly smaller and muted; an [`Element`]
/// title is displayed as-is.
pub fn group<'a, T, Theme, Renderer>(
    title: impl Into<Content<'a, Theme, Renderer>>,
    entries: impl IntoIterator<Item = impl Into<Entry<'a, T, Theme, Renderer>>>,
) -> Group<'a, T, Theme, Renderer> {
    Group::new(entries).title(title)
}

/// Creates an untitled [`Group`] containing a single entry that clears
/// the current selection — the "None" item of native menus, usually
/// placed at the top.
///
/// Selecting it produces the [`PickList::on_deselect`] message, and it
/// displays the check indicator while nothing is selected. Without an
/// [`PickList::on_deselect`] handler, the entry is disabled.
pub fn deselect<'a, T, Theme, Renderer>(
    label: impl Into<Content<'a, Theme, Renderer>>,
) -> Group<'a, T, Theme, Renderer> {
    Group {
        title: None,
        entries: vec![Entry::Deselect(label.into())],
    }
}

/// Creates a selectable option [`Entry`] with the given value.
///
/// This is only needed to disable an option inline via
/// [`Entry::disabled`]; plain values convert into entries on their own.
pub fn option<'a, T, Theme, Renderer>(
    value: T,
) -> Entry<'a, T, Theme, Renderer> {
    Entry::from(value)
}

/// Where the menu of a [`PickList`] is anchored when it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    /// The menu overlays the [`PickList`], with the selected option
    /// aligned on top of it — like native menus on macOS.
    ///
    /// Falls back to [`Anchor::Auto`] when nothing is selected.
    ///
    /// This is the default.
    #[default]
    Selected,
    /// The menu opens below the [`PickList`], or above it when there is
    /// not enough space under it.
    Auto,
    /// The menu always opens above the [`PickList`].
    Top,
    /// The menu always opens below the [`PickList`].
    Bottom,
}

/// The content a [`PickList`] displays for an option, both in its menu
/// and, when the option is selected, in the pick list itself.
///
/// The view function given to [`PickList::new`] can return anything that
/// converts into [`Content`]: a `String` or `&str` for plain text, or an
/// [`Element`] for arbitrary widgets.
pub enum Content<'a, Theme = crate::Theme, Renderer = crate::Renderer> {
    /// Plain text, drawn with the text settings of the [`PickList`].
    Text(String),
    /// An arbitrary message-less widget.
    ///
    /// Content is display-only — it is drawn, but never receives events —
    /// so its elements carry [`Infallible`] as their message and can
    /// never produce one. Keyboard typeahead does not match options
    /// displayed as elements unless [`PickList::typeahead`] is set.
    Element(Element<'a, Infallible, Theme, Renderer>),
}

impl<Theme, Renderer> From<String> for Content<'_, Theme, Renderer> {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl<Theme, Renderer> From<&str> for Content<'_, Theme, Renderer> {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

impl<'a, Theme, Renderer> From<Element<'a, Infallible, Theme, Renderer>>
    for Content<'a, Theme, Renderer>
{
    fn from(element: Element<'a, Infallible, Theme, Renderer>) -> Self {
        Self::Element(element)
    }
}

#[derive(Debug)]
struct State<P: text::Paragraph> {
    menu: menu::State,
    keyboard_modifiers: keyboard::Modifiers,
    is_open: bool,
    is_focused: bool,
    was_focused: bool,
    hovered_option: Option<usize>,
    options: Vec<paragraph::Plain<P>>,
    placeholder: paragraph::Plain<P>,
}

impl<P: text::Paragraph> operation::Focusable for State<P> {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
    }
}

impl<P: text::Paragraph> State<P> {
    /// Creates a new [`State`] for a [`PickList`].
    fn new() -> Self {
        Self {
            menu: menu::State::default(),
            keyboard_modifiers: keyboard::Modifiers::default(),
            is_open: bool::default(),
            is_focused: bool::default(),
            was_focused: bool::default(),
            hovered_option: Option::default(),
            options: Vec::new(),
            placeholder: paragraph::Plain::default(),
        }
    }
}

impl<P: text::Paragraph> Default for State<P> {
    fn default() -> Self {
        Self::new()
    }
}

/// The handle to the right side of the [`PickList`].
#[derive(Debug, Clone, PartialEq)]
pub enum Handle<Font> {
    /// Displays an arrow icon (▼).
    ///
    /// This is the default.
    Arrow {
        /// Font size of the content.
        size: Option<Pixels>,
    },
    /// A custom static handle.
    Static(Icon<Font>),
    /// A custom dynamic handle.
    Dynamic {
        /// The [`Icon`] used when [`PickList`] is closed.
        closed: Icon<Font>,
        /// The [`Icon`] used when [`PickList`] is open.
        open: Icon<Font>,
    },
    /// No handle will be shown.
    None,
}

impl<Font> Default for Handle<Font> {
    fn default() -> Self {
        Self::Arrow { size: None }
    }
}

/// The icon of a [`Handle`].
#[derive(Debug, Clone, PartialEq)]
pub struct Icon<Font> {
    /// Font that will be used to display the `code_point`,
    pub font: Font,
    /// The unicode code point that will be used as the icon.
    pub code_point: char,
    /// Font size of the content.
    pub size: Option<Pixels>,
    /// Line height of the content.
    pub line_height: Option<text::LineHeight>,
    /// The shaping strategy of the icon.
    pub shaping: text::Shaping,
}

/// The possible status of a [`PickList`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`PickList`] can be interacted with.
    Active,
    /// The [`PickList`] is being hovered.
    Hovered,
    /// The [`PickList`] has keyboard focus.
    Focused {
        /// Whether the [`PickList`] is hovered, while focused.
        is_hovered: bool,
    },
    /// The [`PickList`] is open.
    Opened {
        /// Whether the [`PickList`] is hovered, while open.
        is_hovered: bool,
    },
    /// The [`PickList`] is disabled.
    Disabled,
}

/// The appearance of a pick list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The text [`Color`] of the pick list.
    pub text_color: Color,
    /// The placeholder [`Color`] of the pick list.
    pub placeholder_color: Color,
    /// The handle [`Color`] of the pick list.
    pub handle_color: Color,
    /// The [`Background`] of the pick list.
    pub background: Background,
    /// The [`Border`] of the pick list.
    pub border: Border,
}

/// The theme catalog of a [`PickList`].
pub trait Catalog: menu::Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> <Self as Catalog>::Class<'a>;

    /// The default class for the menu of the [`PickList`].
    fn default_menu<'a>() -> <Self as menu::Catalog>::Class<'a> {
        <Self as menu::Catalog>::default()
    }

    /// The [`Style`] of a class with the given status.
    fn style(
        &self,
        class: &<Self as Catalog>::Class<'_>,
        status: Status,
    ) -> Style;
}

/// A styling function for a [`PickList`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default)
    }

    fn style(&self, class: &StyleFn<'_, Self>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of the field of a [`PickList`].
///
/// The border stays neutral; hovering or opening raises the background
/// one step instead of recoloring the outline. For the classic iced
/// styling, see [`legacy`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let active = Style {
        text_color: palette.background.weak.text,
        background: palette.background.weak.color.into(),
        placeholder_color: palette.secondary.base.color,
        handle_color: palette.secondary.base.color,
        border: Border {
            radius: 2.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
    };

    match status {
        Status::Active => active,
        Status::Hovered | Status::Opened { .. } => Style {
            text_color: palette.background.strong.text,
            background: palette.background.strong.color.into(),
            ..active
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.background.strongest.color,
                width: 2.0,
                ..active.border
            },
            ..if is_hovered {
                default(theme, Status::Hovered)
            } else {
                active
            }
        },
        Status::Disabled => Style {
            text_color: palette.background.strongest.color,
            background: palette.background.weaker.color.into(),
            placeholder_color: palette.background.strongest.color,
            handle_color: palette.background.strongest.color,
            border: Border {
                color: palette.background.weak.color,
                ..active.border
            },
        },
    }
}

/// The classic iced style of the field of a [`PickList`]: the border
/// takes the primary color while hovered or open.
pub fn legacy(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let active = Style {
        handle_color: palette.background.weak.text,
        ..default(theme, Status::Active)
    };

    match status {
        Status::Active => active,
        Status::Hovered | Status::Opened { .. } => Style {
            border: Border {
                color: palette.primary.strong.color,
                ..active.border
            },
            ..active
        },
        Status::Focused { .. } => Style {
            border: Border {
                color: palette.primary.strong.color,
                width: 2.0,
                ..active.border
            },
            ..active
        },
        Status::Disabled => default(theme, status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use iced_test::Simulator;
    use iced_test::simulator::click;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Fruit {
        Apple,
        Banana,
        Cherry,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Picked(Fruit),
        Opened,
        Closed,
    }

    fn open(ui: &mut Simulator<'_, Message>) {
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate(click());
    }

    #[test]
    fn arrow_keys_and_enter_select_an_option() {
        let pick = PickList::new(
            None::<Fruit>,
            [Fruit::Apple, Fruit::Banana, Fruit::Cherry],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowUp));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Apple)]);
    }

    #[test]
    fn keyboard_navigation_skips_titles_and_disabled_options() {
        let pick = PickList::new(
            None::<Fruit>,
            options![
                group(
                    "Fruits",
                    [option(Fruit::Apple).disabled(), option(Fruit::Banana)],
                ),
                group("Berries", [option(Fruit::Cherry)]),
            ]
            .spacing(8),
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked)
        .separator(true);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        // Apple is disabled, so the first ArrowDown highlights Banana
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Banana)]);
    }

    #[test]
    fn escape_closes_the_menu_without_selecting() {
        let pick = PickList::new(
            None::<Fruit>,
            [Fruit::Apple, Fruit::Banana],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked)
        .on_open(Message::Opened)
        .on_close(Message::Closed);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Escape));
        // Escape keeps focus, so Enter reopens the menu — but the
        // highlight was reset and nothing gets selected
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Opened, Message::Closed, Message::Opened]
        );
    }

    #[test]
    fn typeahead_highlights_the_first_matching_option() {
        let pick = PickList::new(
            None::<Fruit>,
            [Fruit::Apple, Fruit::Banana, Fruit::Cherry],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        let _ = ui.typewrite("ch");
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Cherry)]);
    }

    #[test]
    fn focused_pick_list_reopens_with_keyboard() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Message {
            Picked(Fruit),
            Focused,
            Opened,
            Closed,
        }

        let pick: PickList<
            '_,
            Fruit,
            _,
            Message,
            crate::Theme,
            crate::Renderer,
        > = PickList::new(
            None::<Fruit>,
            [Fruit::Apple, Fruit::Banana],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked)
        .on_focus(Message::Focused)
        .on_open(Message::Opened)
        .on_close(Message::Closed);

        let mut ui = Simulator::new(pick);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate(click());
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Escape));
        // still focused: Enter reopens the menu
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Focused,
                Message::Opened,
                Message::Closed,
                Message::Opened,
                Message::Picked(Fruit::Apple),
            ]
        );
    }

    #[test]
    fn pressing_outside_blurs_and_closes() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Message {
            Picked(Fruit),
            Focused,
            Blurred,
            Opened,
            Closed,
        }

        let pick: PickList<
            '_,
            Fruit,
            _,
            Message,
            crate::Theme,
            crate::Renderer,
        > = PickList::new(
            None::<Fruit>,
            [Fruit::Apple, Fruit::Banana],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked)
        .on_focus(Message::Focused)
        .on_blur(Message::Blurred)
        .on_open(Message::Opened)
        .on_close(Message::Closed);

        let mut ui = Simulator::new(pick);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate(click());

        ui.point_at(Point::new(900.0, 700.0));
        let _ = ui.simulate(click());

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Focused,
                Message::Opened,
                Message::Blurred,
                Message::Closed,
            ]
        );
    }

    #[test]
    fn deselect_entry_clears_the_selection() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Message {
            Picked(Fruit),
            Cleared,
        }

        let entries = || {
            options![None, [Fruit::Apple, Fruit::Banana, Fruit::Cherry],]
                .spacing(4)
        };

        // with a selection, Home moves to the None entry
        let pick: PickList<
            '_,
            Fruit,
            _,
            Message,
            crate::Theme,
            crate::Renderer,
        > = PickList::new(Some(Fruit::Banana), entries(), |fruit| {
            format!("{fruit:?}")
        })
        .on_select(Message::Picked)
        .on_deselect(Message::Cleared);

        let mut ui = Simulator::new(pick);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate(click());
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Home));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Cleared]);

        // with no selection, the None entry is highlighted on open
        let pick: PickList<
            '_,
            Fruit,
            _,
            Message,
            crate::Theme,
            crate::Renderer,
        > = PickList::new(None::<Fruit>, entries(), |fruit| {
            format!("{fruit:?}")
        })
        .on_select(Message::Picked)
        .on_deselect(Message::Cleared);

        let mut ui = Simulator::new(pick);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate(click());
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Cleared]);
    }

    #[test]
    fn deselect_entry_is_disabled_without_a_handler() {
        let pick = PickList::new(
            None::<Fruit>,
            options![deselect("None"), [Fruit::Apple, Fruit::Banana],],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        // Home skips the disabled None entry and lands on Apple
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Home));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Apple)]);
    }

    #[test]
    fn element_titles_and_placeholder_are_display_only() {
        use iced_widget::Widget as _;

        let pick = PickList::new(
            None::<Fruit>,
            options![group(
                crate::text("FRUITS").size(10).boxed(),
                [option(Fruit::Apple), option(Fruit::Banana)],
            )],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked)
        .placeholder(crate::text("Pick one...").size(13).boxed());

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        // the title row is skipped: ArrowDown lands on Apple
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Apple)]);
    }

    #[test]
    fn item_aligned_menu_selects_with_keyboard() {
        let pick = PickList::new(
            Some(Fruit::Banana),
            [Fruit::Apple, Fruit::Banana, Fruit::Cherry],
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        // the highlight starts on the selected Banana
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::ArrowDown));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Cherry)]);
    }

    #[test]
    fn shrunk_menu_still_selects_options() {
        let pick = PickList::new(
            None::<Fruit>,
            options![
                group("Fruits", [option(Fruit::Apple), option(Fruit::Banana)]),
                group("Berries", [option(Fruit::Cherry)]),
            ]
            .spacing(8),
            |fruit| format!("{fruit:?}"),
        )
        .on_select(Message::Picked)
        .menu_width(Length::Shrink)
        .separator(true);

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        let _ = ui.tap_key(keyboard::Key::Named(key::Named::End));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Cherry)]);
    }

    #[test]
    fn keyboard_navigation_scrolls_the_menu() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Message {
            Picked(&'static str),
        }

        let options = [
            "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf",
            "hotel", "india", "juliett",
        ];

        let pick: PickList<'_, _, _, Message, crate::Theme, crate::Renderer> =
            PickList::new(None::<&'static str>, options, |name| *name)
                .on_select(Message::Picked)
                .menu_height(80);

        let mut ui = Simulator::new(pick);
        ui.point_at(Point::new(10.0, 10.0));
        let _ = ui.simulate(click());

        let _ = ui.tap_key(keyboard::Key::Named(key::Named::End));
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked("juliett")]);
    }

    #[test]
    fn element_content_options_can_be_selected() {
        use iced_widget::Widget as _;

        let pick = PickList::new(
            None::<Fruit>,
            [Fruit::Apple, Fruit::Banana, Fruit::Cherry],
            |fruit| match fruit {
                Fruit::Banana => {
                    Content::Element(crate::text("Banana!").boxed())
                }
                other => Content::Text(format!("{other:?}")),
            },
        )
        .on_select(Message::Picked)
        .typeahead(|fruit| format!("{fruit:?}"));

        let mut ui = Simulator::new(pick);
        open(&mut ui);

        let _ = ui.typewrite("ba");
        let _ = ui.tap_key(keyboard::Key::Named(key::Named::Enter));

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Picked(Fruit::Banana)]);
    }
}
