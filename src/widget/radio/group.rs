//! A focus-managed radio group.
//!
//! [`Group`] is the WAI-ARIA "radiogroup": the whole set of buttons is a
//! single tab stop, and once focused the arrow keys move a roving focus
//! between the enabled options while selection follows along. It is the
//! widget the top-level [`radio`](super::radio) function builds; reach for
//! the [`Single`](super::single::Single) escape hatch only when you need a
//! lone button in a bespoke layout.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use sweeten::widget::radio;
//!
//! struct State {
//!    selection: Option<Choice>,
//! }
//!
//! #[derive(Debug, Clone, Copy)]
//! enum Message {
//!     Selected(Choice),
//! }
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum Choice {
//!     A,
//!     B,
//!     All,
//! }
//!
//! impl Choice {
//!     const ALL: [Choice; 3] = [Choice::A, Choice::B, Choice::All];
//!
//!     fn label(self) -> &'static str {
//!         match self {
//!             Choice::A => "A",
//!             Choice::B => "B",
//!             Choice::All => "All of the above",
//!         }
//!     }
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     radio(state.selection, Choice::ALL, |choice| choice.label())
//!         .on_select(Message::Selected)
//!         .spacing(14.0)
//!         .into()
//! }
//! ```
use crate::animation::cubic_bezier;
use crate::core::alignment;
use crate::core::animation::Easing;
use crate::core::keyboard;
use crate::core::keyboard::key;
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::time::Instant;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Animation, Color, Element, Event, Layout, Length, Pixels, Point, Rectangle,
    Shell, Size, Widget,
};
use crate::widget::focus;

use super::content::Content;
use super::dot;
use super::style::{Catalog, Status, Style, StyleFn};

/// The boxed callback producing a message from a selected value.
type SelectFn<'a, V, Message> = Box<dyn Fn(V) -> Message + 'a>;

/// The boxed predicate marking individual options as disabled.
type DisabledFn<'a, V> = Box<dyn Fn(&V) -> bool + 'a>;

/// A set of radio buttons managed as a single focusable group.
///
/// This is a sweeten-only widget with no upstream counterpart. Unlike a
/// stack of [`Single`](super::single::Single) buttons — each its own tab
/// stop — a [`Group`] is one tab stop for the whole set (the WAI-ARIA
/// "radiogroup" pattern): tabbing in lands on the selected option, the
/// arrow keys rove between the enabled options, and selection follows
/// focus. Each button animates identically to the [`Single`], reusing the
/// shared [`style`](super::style) and [`dot`](super::dot) machinery.
///
/// The [`Group`] is disabled until [`on_select`](Self::on_select) is called
/// to set the message produced when an option is chosen.
pub struct Group<
    'a,
    V,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    selected: Option<V>,
    options: Vec<V>,
    content: Vec<Content<'a, Theme, Renderer>>,
    on_select: Option<SelectFn<'a, V, Message>>,
    is_disabled: Option<DisabledFn<'a, V>>,
    on_focus: Option<Message>,
    on_blur: Option<Message>,
    id: Option<widget::Id>,
    width: Length,
    horizontal: bool,
    size: f32,
    spacing: Option<f32>,
    gap: f32,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
    shaping: text::Shaping,
    font: Option<Renderer::Font>,
    class: Theme::Class<'a>,
}

impl<'a, V, Message, Theme, Renderer> Group<'a, V, Message, Theme, Renderer>
where
    V: Eq + Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// The default size of each radio button in a [`Group`].
    pub const DEFAULT_SIZE: f32 = 16.0;

    /// The default spacing between options in a vertical [`Group`].
    pub const DEFAULT_SPACING: f32 = 8.0;

    /// The default spacing between options in a horizontal [`Group`],
    /// wider than the vertical default so each label stays visually bound
    /// to its own button rather than drifting toward the next option.
    pub const DEFAULT_HORIZONTAL_SPACING: f32 = 16.0;

    /// The default gap between a button and its label. Tighter than the
    /// inter-option spacing so the circle and its label read as one unit.
    pub const DEFAULT_GAP: f32 = 6.0;

    /// Creates a new [`Group`] from the current `selected` value, the set of
    /// `options`, and a `view` function producing each option's label
    /// [`Content`].
    ///
    /// The [`Group`] is disabled until [`on_select`](Self::on_select) is
    /// called; the `selected` value may be `None`, which is the usual state
    /// on init.
    pub fn new<T>(
        selected: Option<V>,
        options: impl IntoIterator<Item = V>,
        view: impl Fn(&V) -> T,
    ) -> Self
    where
        T: Into<Content<'a, Theme, Renderer>>,
    {
        let options: Vec<V> = options.into_iter().collect();
        let content = options.iter().map(|value| view(value).into()).collect();

        Group {
            selected,
            options,
            content,
            on_select: None,
            is_disabled: None,
            on_focus: None,
            on_blur: None,
            id: None,
            width: Length::Shrink,
            horizontal: false,
            size: Self::DEFAULT_SIZE,
            spacing: None,
            gap: Self::DEFAULT_GAP,
            text_size: None,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::default(),
            font: None,
            class: Theme::default(),
        }
    }

    /// Sets the function called when an option is selected. It receives the
    /// value of the chosen option and must produce a `Message`.
    ///
    /// Unless `on_select` is called, the [`Group`] will be disabled.
    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(V) -> Message,
    {
        self.on_select = Some(Box::new(f));
        self
    }

    /// Sets the function called when an option is selected, if `Some`.
    ///
    /// If `None`, the [`Group`] will be disabled.
    pub fn on_select_maybe<F>(mut self, f: Option<F>) -> Self
    where
        F: 'a + Fn(V) -> Message,
    {
        self.on_select = f.map(|f| Box::new(f) as _);
        self
    }

    /// Sets a predicate that marks individual options as disabled. Disabled
    /// options are skipped by keyboard navigation and cannot be selected.
    pub fn disabled<F>(mut self, is_disabled: F) -> Self
    where
        F: 'a + Fn(&V) -> bool,
    {
        self.is_disabled = Some(Box::new(is_disabled));
        self
    }

    /// Sets the message produced when the [`Group`] gains keyboard focus.
    pub fn on_focus(mut self, on_focus: Message) -> Self {
        self.on_focus = Some(on_focus);
        self
    }

    /// Sets the message produced when the [`Group`] loses keyboard focus.
    pub fn on_blur(mut self, on_blur: Message) -> Self {
        self.on_blur = Some(on_blur);
        self
    }

    /// Sets the [`widget::Id`] of the [`Group`], for programmatic focus.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the width of the [`Group`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Lays the options out in a row instead of a column.
    ///
    /// Keyboard navigation is unchanged: the arrow keys rove between the
    /// enabled options in either orientation.
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Sets the size of each radio button in the [`Group`].
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the spacing between options in the [`Group`] — the same meaning
    /// as spacing on a [`Column`](crate::widget::Column) or
    /// [`Row`](crate::widget::Row).
    ///
    /// When left unset, this defaults to
    /// [`DEFAULT_SPACING`](Self::DEFAULT_SPACING) in a vertical group and
    /// the wider
    /// [`DEFAULT_HORIZONTAL_SPACING`](Self::DEFAULT_HORIZONTAL_SPACING) in a
    /// horizontal one.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = Some(spacing.into().0);
        self
    }

    /// Sets the gap between each button and its label, defaulting to
    /// [`DEFAULT_GAP`](Self::DEFAULT_GAP).
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into().0;
        self
    }

    /// Sets the text size of the labels in the [`Group`].
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the [`text::LineHeight`] of the labels in the [`Group`].
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the labels in the [`Group`].
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the text font of the labels in the [`Group`].
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the style of the [`Group`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Group`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Whether the option at `index` matches the current selection.
    fn is_selected(&self, index: usize) -> bool {
        self.selected.as_ref() == self.options.get(index)
    }

    /// Whether the option at `index` can be interacted with.
    fn is_enabled(&self, index: usize) -> bool {
        self.on_select.is_some()
            && self.options.get(index).is_some_and(|value| {
                self.is_disabled.as_ref().is_none_or(|f| !f(value))
            })
    }

    /// The first enabled option, if any.
    fn first_enabled(&self) -> Option<usize> {
        (0..self.options.len()).find(|&i| self.is_enabled(i))
    }

    /// The index the currently-selected option occupies, if enabled.
    fn selected_index(&self) -> Option<usize> {
        (0..self.options.len())
            .find(|&i| self.is_selected(i) && self.is_enabled(i))
    }

    /// Where focus should land when the [`Group`] is focused: the selected
    /// option if enabled, otherwise the first enabled option.
    fn initial_focus(&self) -> Option<usize> {
        self.selected_index().or_else(|| self.first_enabled())
    }

    /// The next enabled option after `from`, wrapping around, moving forward
    /// or backward. Returns `None` only when no option is enabled.
    fn step(&self, from: usize, forward: bool) -> Option<usize> {
        let n = self.options.len();

        if n == 0 {
            return None;
        }

        let delta = if forward { 1 } else { -1 };

        (1..=n as i64).find_map(|offset| {
            let idx =
                (from as i64 + delta * offset).rem_euclid(n as i64) as usize;

            self.is_enabled(idx).then_some(idx)
        })
    }

    /// The enabled option currently under the cursor, if any.
    fn item_under(
        &self,
        cursor: mouse::Cursor,
        layout: Layout<'_>,
    ) -> Option<usize> {
        layout
            .children()
            .enumerate()
            .find(|(i, row)| {
                cursor.is_over(row.bounds()) && self.is_enabled(*i)
            })
            .map(|(i, _)| i)
    }

    /// Grows or shrinks the per-item state to match the current options,
    /// seeding freshly-added items to their current selection.
    fn reconcile(&self, state: &mut State<Renderer::Paragraph>) {
        let n = self.options.len();

        state.items.truncate(n);

        while state.items.len() < n {
            let i = state.items.len();
            state.items.push(ItemState::new(self.is_selected(i)));
        }

        if state.focused.is_some_and(|i| i >= n) {
            state.focused = None;
        }
    }
}

/// Per-option animation and text state owned by the [`Group`].
struct ItemState<P: text::Paragraph> {
    paragraph: widget::text::State<P>,
    animation: Animation<bool>,
    last_is_selected: bool,
}

impl<P: text::Paragraph> ItemState<P> {
    fn new(is_selected: bool) -> Self {
        ItemState {
            paragraph: widget::text::State::default(),
            animation: Animation::new(is_selected)
                .very_quick()
                // cubic-bezier(0, 0, 0.2, 1) — Tailwind v4's `--ease-out`,
                // the same curve the single radio and checkbox use.
                .easing(Easing::Custom(|t| {
                    cubic_bezier(0.0, 0.0, 0.2, 1.0, t)
                })),
            last_is_selected: is_selected,
        }
    }
}

/// Group-level state: the roving focus, animation clock, and press-tracking.
struct State<P: text::Paragraph> {
    items: Vec<ItemState<P>>,
    now: Option<Instant>,
    focus: Option<focus::Source>,
    was_focused: bool,
    focused: Option<usize>,
    pressed: Option<usize>,
}

impl<P: text::Paragraph> operation::Focusable for State<P> {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        // Only focus operations (Tab / programmatic) call this, so it is
        // always keyboard-style focus that should show the ring.
        self.focus = Some(focus::Source::Keyboard);
    }

    fn unfocus(&mut self) {
        self.focus = None;
        self.focused = None;
    }
}

impl<V, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Group<'_, V, Message, Theme, Renderer>
where
    V: Eq + Clone,
    Message: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph> {
            items: (0..self.options.len())
                .map(|i| ItemState::new(self.is_selected(i)))
                .collect(),
            now: None,
            focus: None,
            was_focused: false,
            focused: None,
            pressed: None,
        })
    }

    fn diff(&mut self, tree: &mut Tree) {
        {
            let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
            self.reconcile(state);
        }

        tree.diff_children_custom(
            &mut self.content,
            |tree, content| {
                if let Content::Element(element) = content {
                    tree.diff(element.as_widget_mut());
                }
            },
            |content| match content {
                Content::Element(element) => Tree::new(element.as_widget()),
                Content::Text(_) => Tree::empty(),
            },
        );
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
    ) -> layout::Node {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State<Renderer::Paragraph>>();
        self.reconcile(state);

        let size = self.size;
        let gap = self.gap;
        let spacing = self.spacing.unwrap_or(if self.horizontal {
            Self::DEFAULT_HORIZONTAL_SPACING
        } else {
            Self::DEFAULT_SPACING
        });
        let text_size = self.text_size;
        let line_height = self.line_height;
        let font = self.font;
        let shaping = self.shaping;

        let limits = limits.width(self.width);

        let horizontal = self.horizontal;
        let mut nodes = Vec::with_capacity(self.content.len());
        // `main` advances along the layout axis (down for a column, across
        // for a row); `cross` tracks the widest / tallest option.
        let mut main = 0.0;
        let mut cross = 0.0_f32;

        for ((content, item), child) in self
            .content
            .iter_mut()
            .zip(state.items.iter_mut())
            .zip(children.iter_mut())
        {
            let row = layout::next_to_each_other(
                &limits,
                gap,
                |_| layout::Node::new(Size::new(size, size)),
                |limits| match content {
                    Content::Text(fragment) => widget::text::layout(
                        &mut item.paragraph,
                        renderer,
                        limits,
                        fragment,
                        widget::text::Format {
                            width: Length::Shrink,
                            height: Length::Shrink,
                            line_height,
                            size: text_size,
                            font,
                            align_x: text::Alignment::Default,
                            align_y: alignment::Vertical::Top,
                            shaping,
                            wrapping: text::Wrapping::default(),
                            ellipsis: text::Ellipsis::None,
                        },
                    ),
                    Content::Element(element) => {
                        element.as_widget_mut().layout(child, renderer, limits)
                    }
                },
            );

            let row_size = row.size();

            if horizontal {
                cross = cross.max(row_size.height);
                nodes.push(row.move_to(Point::new(main, 0.0)));
                main += row_size.width + spacing;
            } else {
                cross = cross.max(row_size.width);
                nodes.push(row.move_to(Point::new(0.0, main)));
                main += row_size.height + spacing;
            }
        }

        // Trim the trailing inter-option spacing off the main axis.
        let extent = (main - spacing).max(0.0);
        let intrinsic = if horizontal {
            Size::new(extent, cross)
        } else {
            Size::new(cross, extent)
        };

        layout::Node::with_children(
            limits.resolve(self.width, Length::Shrink, intrinsic),
            nodes,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
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
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        self.reconcile(state);

        // Animation bookkeeping — advance the clock and keep the frame loop
        // alive while any button is animating.
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = Some(*now);

            if state
                .items
                .iter()
                .any(|item| item.animation.is_animating(*now))
            {
                shell.request_redraw();
            }
        }

        // Kick off a fade whenever an option's selection flips.
        for i in 0..state.items.len() {
            let is_selected = self.is_selected(i);
            let item = &mut state.items[i];

            if is_selected != item.last_is_selected {
                item.last_is_selected = is_selected;

                if let Some(now) = state.now {
                    item.animation.go_mut(is_selected, now);
                    shell.request_redraw();
                }
            }
        }

        // React to focus changes coming from operations (e.g. focus_next):
        // land the roving focus and publish on_focus / on_blur.
        let is_focused = state.focus.is_some();
        if is_focused != state.was_focused {
            if is_focused {
                if state.focused.is_none() {
                    state.focused = self.initial_focus();
                }

                if let Some(on_focus) = &self.on_focus {
                    shell.publish(on_focus.clone());
                }
            } else if let Some(on_blur) = &self.on_blur {
                shell.publish(on_blur.clone());
            }

            state.was_focused = is_focused;
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if self.on_select.is_some() =>
            {
                if let Some(i) = self.item_under(cursor, layout) {
                    if state.focus.is_none() {
                        state.was_focused = true;

                        if let Some(on_focus) = &self.on_focus {
                            shell.publish(on_focus.clone());
                        }
                    }

                    // Clicking focuses the group for keyboard use, but is a
                    // pointer interaction, so it must not paint the ring.
                    state.focus = Some(focus::Source::Mouse);
                    state.focused = Some(i);
                    state.pressed = Some(i);
                    shell.capture_event();
                } else if state.focus.is_some()
                    && !cursor.is_over(layout.bounds())
                {
                    state.focus = None;
                    state.was_focused = false;
                    state.focused = None;

                    if let Some(on_blur) = &self.on_blur {
                        shell.publish(on_blur.clone());
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                if let Some(i) = state.pressed.take()
                    && self.item_under(cursor, layout) == Some(i)
                    && let Some(on_select) = &self.on_select
                {
                    shell.publish(on_select(self.options[i].clone()));
                    shell.capture_event();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(named),
                ..
            }) if state.focus.is_some() && self.on_select.is_some() => {
                // Arrow / Space navigation re-arms keyboard focus so the
                // ring reappears after a prior mouse click (focus-visible).
                if matches!(
                    named,
                    key::Named::ArrowDown
                        | key::Named::ArrowRight
                        | key::Named::ArrowUp
                        | key::Named::ArrowLeft
                        | key::Named::Space
                ) {
                    state.focus = Some(focus::Source::Keyboard);
                }

                let on_select = self.on_select.as_ref().unwrap();

                match named {
                    key::Named::ArrowDown | key::Named::ArrowRight => {
                        let from =
                            state.focused.or_else(|| self.first_enabled());

                        if let Some(next) =
                            from.and_then(|from| self.step(from, true))
                        {
                            state.focused = Some(next);
                            shell
                                .publish(on_select(self.options[next].clone()));
                            shell.capture_event();
                        }
                    }
                    key::Named::ArrowUp | key::Named::ArrowLeft => {
                        let from =
                            state.focused.or_else(|| self.first_enabled());

                        if let Some(prev) =
                            from.and_then(|from| self.step(from, false))
                        {
                            state.focused = Some(prev);
                            shell
                                .publish(on_select(self.options[prev].clone()));
                            shell.capture_event();
                        }
                    }
                    key::Named::Space => {
                        if let Some(i) = state.focused
                            && self.is_enabled(i)
                            && !self.is_selected(i)
                        {
                            shell.publish(on_select(self.options[i].clone()));
                            shell.capture_event();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.item_under(cursor, layout).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        // The focus ring reuses the accent the selected style paints its
        // border with, so it reads correctly under any theme.
        let ring_color = theme
            .style(&self.class, Status::Active { is_selected: true })
            .border_color;

        for (i, row) in layout.children().enumerate() {
            let mut cells = row.children();
            let dot_layout = cells.next().unwrap();
            let label_layout = cells.next().unwrap();

            let item = &state.items[i];
            let is_selected = self.is_selected(i);

            let status = if !self.is_enabled(i) {
                Status::Disabled { is_selected }
            } else if state.focused == Some(i) || cursor.is_over(row.bounds()) {
                Status::Hovered { is_selected }
            } else {
                Status::Active { is_selected }
            };

            // Interpolate the fill, border, and dot colors while the dot
            // fades in, matching the single radio button.
            let style = match state.now {
                Some(now) if item.animation.is_animating(now) => {
                    let off =
                        theme.style(&self.class, status.with_selected(false));
                    let on =
                        theme.style(&self.class, status.with_selected(true));

                    dot::blend(&off, &on, &item.animation, now)
                }
                _ => theme.style(&self.class, status),
            };

            if state.focus == Some(focus::Source::Keyboard)
                && state.focused == Some(i)
            {
                draw_focus_ring(renderer, dot_layout.bounds(), ring_color);
            }

            let progress =
                dot::progress(&item.animation, state.now, is_selected);

            dot::draw(renderer, dot_layout.bounds(), &style, progress);

            match &self.content[i] {
                Content::Text(_) => {
                    crate::text::draw(
                        renderer,
                        defaults,
                        label_layout.bounds(),
                        item.paragraph.raw(),
                        crate::text::Style {
                            color: style.text_color,
                        },
                        viewport,
                    );
                }
                Content::Element(element) => {
                    element.as_widget().draw(
                        &tree.children[i],
                        renderer,
                        theme,
                        defaults,
                        label_layout,
                        cursor,
                        viewport,
                    );
                }
            }
        }
    }
}

/// Draws a soft circular halo hugging the focused option's dot.
///
/// This is the analog of shadcn's `focus-visible:ring/50` glow around the
/// control itself — a semi-transparent band right outside the circle, not
/// a hard outline wrapping the whole row. `bounds` is the dot's bounds.
fn draw_focus_ring<Renderer: crate::core::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    color: Color,
) {
    // Hug the dot as a full circle; `focus::ring` adds the band gap.
    focus::ring(renderer, bounds, bounds.height / 2.0, color);
}

impl<'a, V, Message, Theme, Renderer>
    From<Group<'a, V, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    V: 'a + Eq + Clone,
    Message: 'a + Clone,
    Theme: 'a + Catalog,
    Renderer: 'a + text::Renderer,
{
    fn from(
        group: Group<'a, V, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(group)
    }
}
