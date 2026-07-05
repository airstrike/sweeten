//! A single circular radio button.
//!
//! This is the classic one-button-per-value widget, tucked behind
//! [`radio::single`](self) as an escape hatch for custom layouts. Most
//! callers want the focus-managing [`Group`](super::group::Group), which
//! the top-level [`radio`](super::radio) function builds.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::column;
//! use sweeten::widget::radio::Single;
//!
//! struct State {
//!    selection: Option<Choice>,
//! }
//!
//! #[derive(Debug, Clone, Copy)]
//! enum Message {
//!     RadioSelected(Choice),
//! }
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum Choice {
//!     A,
//!     B,
//!     All,
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     let a = Single::new(Choice::A, state.selection)
//!         .label("A")
//!         .on_toggle(Message::RadioSelected);
//!
//!     let b = Single::new(Choice::B, state.selection)
//!         .label("B")
//!         .on_toggle(Message::RadioSelected);
//!
//!     let all = Single::new(Choice::All, state.selection)
//!         .label("All of the above")
//!         .on_toggle(Message::RadioSelected);
//!
//!     column![a, b, all].into()
//! }
//! ```
use crate::animation::cubic_bezier;
use crate::core::alignment;
use crate::core::animation::Easing;
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::time::Instant;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Animation, Element, Event, Layout, Length, Pixels, Rectangle, Shell, Size,
    Widget,
};

use super::dot;
use super::style::{Catalog, Status, Style, StyleFn};

/// A circular button representing a choice.
///
/// This is a sweetened version of [`iced`'s `radio`] with a smooth
/// animation when the selection changes — the dot fades and scales in
/// (or out) while the fill and border colors interpolate in unison,
/// instead of the dot snapping the moment the selection flips.
///
/// Unlike upstream, the click callback is a builder — the [`Single`] is
/// disabled until [`on_toggle`] is called — and the selection may be
/// `None`, which is the usual state on init.
///
/// [`iced`'s `radio`]: https://docs.iced.rs/iced/widget/radio/index.html
/// [`on_toggle`]: Single::on_toggle
pub struct Single<
    'a,
    V,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    value: V,
    is_selected: bool,
    on_toggle: Option<Box<dyn Fn(V) -> Message + 'a>>,
    label: Option<text::Fragment<'a>>,
    width: Length,
    size: f32,
    gap: f32,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
    shaping: text::Shaping,
    wrapping: text::Wrapping,
    font: Option<Renderer::Font>,
    class: Theme::Class<'a>,
    last_status: Option<Status>,
}

impl<'a, V, Message, Theme, Renderer> Single<'a, V, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// The default size of a [`Single`] radio button.
    pub const DEFAULT_SIZE: f32 = 16.0;

    /// The default gap between a [`Single`] radio button and its label.
    pub const DEFAULT_GAP: f32 = 6.0;

    /// Creates a new [`Single`] radio button.
    ///
    /// It expects:
    ///   * the value related to the radio button
    ///   * the current selected value, if any
    ///
    /// The [`Single`] is disabled until [`on_toggle`](Self::on_toggle) is
    /// called to set the message produced when it is clicked.
    pub fn new(value: V, selected: Option<V>) -> Self
    where
        V: Eq,
    {
        Single {
            is_selected: selected.as_ref() == Some(&value),
            value,
            on_toggle: None,
            label: None,
            width: Length::Shrink,
            size: Self::DEFAULT_SIZE,
            gap: Self::DEFAULT_GAP,
            text_size: None,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::default(),
            wrapping: text::Wrapping::default(),
            font: None,
            class: Theme::default(),
            last_status: None,
        }
    }

    /// Sets the label of the [`Single`] radio button.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Sets the function that will be called when the [`Single`] is
    /// clicked. It will receive the value of the radio button and must
    /// produce a `Message`.
    ///
    /// Unless `on_toggle` is called, the [`Single`] will be disabled.
    pub fn on_toggle<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(V) -> Message,
    {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Sets the function that will be called when the [`Single`] is
    /// clicked, if `Some`.
    ///
    /// If `None`, the [`Single`] will be disabled.
    pub fn on_toggle_maybe<F>(mut self, f: Option<F>) -> Self
    where
        F: 'a + Fn(V) -> Message,
    {
        self.on_toggle = f.map(|f| Box::new(f) as _);
        self
    }

    /// Sets the size of the [`Single`] radio button.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the width of the [`Single`] radio button.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the gap between the [`Single`] radio button and its label.
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into().0;
        self
    }

    /// Sets the text size of the [`Single`] radio button.
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Single`] radio button.
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Single`] radio button.
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the [`text::Wrapping`] strategy of the [`Single`] radio button.
    pub fn wrapping(mut self, wrapping: text::Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Sets the text font of the [`Single`] radio button.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the style of the [`Single`] radio button.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Single`] radio button.
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

/// Internal state for the animated [`Single`].
struct State<P: text::Paragraph> {
    paragraph: widget::text::State<P>,
    animation: Animation<bool>,
    now: Option<Instant>,
    last_is_selected: bool,
    /// Whether the most recent left-button / finger press landed inside
    /// the radio. The click fires on release, gated by this flag plus a
    /// fresh "still inside on release" bounds check — pressing outside
    /// and dragging in (or pressing in and dragging out before release)
    /// must not fire.
    is_pressed: bool,
}

impl<V, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Single<'_, V, Message, Theme, Renderer>
where
    V: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph> {
            paragraph: widget::text::State::default(),
            animation: Animation::new(self.is_selected)
                .very_quick()
                // cubic-bezier(0, 0, 0.2, 1) — Tailwind v4's
                // `--ease-out`. Same curve the checkbox uses.
                .easing(Easing::Custom(|t| {
                    cubic_bezier(0.0, 0.0, 0.2, 1.0, t)
                })),
            now: None,
            last_is_selected: self.is_selected,
            is_pressed: false,
        })
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
        layout::next_to_each_other(
            &limits.width(self.width),
            if self.label.is_some() { self.gap } else { 0.0 },
            |_| layout::Node::new(Size::new(self.size, self.size)),
            |limits| {
                if let Some(label) = self.label.as_deref() {
                    let state =
                        tree.state.downcast_mut::<State<Renderer::Paragraph>>();

                    widget::text::layout(
                        &mut state.paragraph,
                        renderer,
                        limits,
                        label,
                        widget::text::Format {
                            width: self.width,
                            height: Length::Shrink,
                            line_height: self.line_height,
                            size: self.text_size,
                            font: self.font,
                            align_x: text::Alignment::Default,
                            align_y: alignment::Vertical::Top,
                            shaping: self.shaping,
                            wrapping: self.wrapping,
                            ellipsis: text::Ellipsis::None,
                        },
                    )
                } else {
                    layout::Node::new(Size::ZERO)
                }
            },
        )
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

        // Animation bookkeeping — runs regardless of whether the widget is
        // enabled, so that in-flight animations complete even if the radio
        // becomes disabled.
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = Some(*now);

            if state.animation.is_animating(*now) {
                shell.request_redraw();
            }
        }

        if self.is_selected != state.last_is_selected {
            state.last_is_selected = self.is_selected;

            if let Some(now) = state.now {
                state.animation.go_mut(self.is_selected, now);
                shell.request_redraw();
            }
        }

        // Fire on release, but only when *both* press and release happened
        // inside the bounds — pressing outside and dragging in, or pressing
        // inside and dragging out before release, must not fire. We track
        // press-inside in `state.is_pressed` and gate the publish on a fresh
        // "still inside" bounds check.
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if self.on_toggle.is_some()
                    && cursor.is_over(layout.bounds()) =>
            {
                state.is_pressed = true;
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                let was_pressed =
                    std::mem::replace(&mut state.is_pressed, false);

                if was_pressed
                    && cursor.is_over(layout.bounds())
                    && let Some(on_toggle) = &self.on_toggle
                {
                    shell.publish((on_toggle)(self.value.clone()));
                    shell.capture_event();
                }
            }
            _ => {}
        }

        let current_status = {
            let is_mouse_over = cursor.is_over(layout.bounds());
            let is_disabled = self.on_toggle.is_none();
            let is_selected = self.is_selected;

            if is_disabled {
                Status::Disabled { is_selected }
            } else if is_mouse_over {
                Status::Hovered { is_selected }
            } else {
                Status::Active { is_selected }
            }
        };

        if let Event::Window(window::Event::RedrawRequested(_now)) = event {
            self.last_status = Some(current_status);
        } else if self
            .last_status
            .is_some_and(|status| status != current_status)
        {
            shell.request_redraw();
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
        if cursor.is_over(layout.bounds()) && self.on_toggle.is_some() {
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
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        let mut children = layout.children();

        let current_status = self.last_status.unwrap_or(Status::Disabled {
            is_selected: self.is_selected,
        });

        // While the selection is animating, interpolate the fill, border,
        // and dot colors between the off- and on-state styles so they fade
        // in sync with the dot rather than snapping when `is_selected`
        // flips.
        let style = match state.now {
            Some(now) if state.animation.is_animating(now) => {
                let off = theme
                    .style(&self.class, current_status.with_selected(false));
                let on = theme
                    .style(&self.class, current_status.with_selected(true));

                dot::blend(&off, &on, &state.animation, now)
            }
            _ => theme.style(&self.class, current_status),
        };

        {
            let layout = children.next().unwrap();
            let progress =
                dot::progress(&state.animation, state.now, self.is_selected);

            dot::draw(renderer, layout.bounds(), &style, progress);
        }

        if self.label.is_none() {
            return;
        }

        {
            let label_layout = children.next().unwrap();

            crate::text::draw(
                renderer,
                defaults,
                label_layout.bounds(),
                state.paragraph.raw(),
                crate::text::Style {
                    color: style.text_color,
                },
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        if let Some(label) = self.label.as_deref() {
            operation.text(None, layout.bounds(), label);
        }
    }
}

impl<'a, V, Message, Theme, Renderer>
    From<Single<'a, V, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    V: 'a + Clone,
    Message: 'a,
    Theme: 'a + Catalog,
    Renderer: 'a + text::Renderer,
{
    fn from(
        radio: Single<'a, V, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(radio)
    }
}
