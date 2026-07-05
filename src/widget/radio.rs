//! Radio buttons let users choose a single option from a bunch of options.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::column;
//! use sweeten::widget::radio;
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
//!     C,
//!     All,
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     let a = radio(Choice::A, state.selection)
//!         .label("A")
//!         .on_toggle(Message::RadioSelected);
//!
//!     let b = radio(Choice::B, state.selection)
//!         .label("B")
//!         .on_toggle(Message::RadioSelected);
//!
//!     let all = radio(Choice::All, state.selection)
//!         .label("All of the above")
//!         .on_toggle(Message::RadioSelected);
//!
//!     column![a, b, all].into()
//! }
//! ```
use crate::animation::cubic_bezier;
use crate::core::alignment;
use crate::core::animation::Easing;
use crate::core::border::{self, Border};
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::theme::palette;
use crate::core::time::Instant;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Animation, Background, Color, Element, Event, Layout, Length, Pixels,
    Rectangle, Shell, Size, Theme, Widget,
};

/// A circular button representing a choice.
///
/// This is a sweetened version of [`iced`'s `radio`] with a smooth
/// animation when the selection changes — the dot fades and scales in
/// (or out) while the fill and border colors interpolate in unison,
/// instead of the dot snapping the moment the selection flips.
///
/// Unlike upstream, the click callback is a builder — the [`Radio`] is
/// disabled until [`on_toggle`] is called — and the selection may be
/// `None`, which is the usual state on init.
///
/// [`iced`'s `radio`]: https://docs.iced.rs/iced/widget/radio/index.html
/// [`on_toggle`]: Radio::on_toggle
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use iced::widget::column;
/// use sweeten::widget::radio;
///
/// struct State {
///    selection: Option<Choice>,
/// }
///
/// #[derive(Debug, Clone, Copy)]
/// enum Message {
///     RadioSelected(Choice),
/// }
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum Choice {
///     A,
///     B,
///     C,
///     All,
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     let a = radio(Choice::A, state.selection)
///         .label("A")
///         .on_toggle(Message::RadioSelected);
///
///     let b = radio(Choice::B, state.selection)
///         .label("B")
///         .on_toggle(Message::RadioSelected);
///
///     let all = radio(Choice::All, state.selection)
///         .label("All of the above")
///         .on_toggle(Message::RadioSelected);
///
///     column![a, b, all].into()
/// }
/// ```
pub struct Radio<
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
    spacing: f32,
    text_size: Option<Pixels>,
    line_height: text::LineHeight,
    shaping: text::Shaping,
    wrapping: text::Wrapping,
    letter_spacing: crate::core::Em,
    font_features: Vec<crate::core::font::Feature>,
    font: Option<Renderer::Font>,
    class: Theme::Class<'a>,
    last_status: Option<Status>,
}

impl<'a, V, Message, Theme, Renderer> Radio<'a, V, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// The default size of a [`Radio`] button.
    pub const DEFAULT_SIZE: f32 = 16.0;

    /// The default spacing of a [`Radio`] button.
    pub const DEFAULT_SPACING: f32 = 8.0;

    /// Creates a new [`Radio`] button.
    ///
    /// It expects:
    ///   * the value related to the [`Radio`] button
    ///   * the current selected value, if any
    ///
    /// The [`Radio`] is disabled until [`on_toggle`](Self::on_toggle) is
    /// called to set the message produced when it is clicked.
    pub fn new(value: V, selected: Option<V>) -> Self
    where
        V: Eq,
    {
        Radio {
            is_selected: selected.as_ref() == Some(&value),
            value,
            on_toggle: None,
            label: None,
            width: Length::Shrink,
            size: Self::DEFAULT_SIZE,
            spacing: Self::DEFAULT_SPACING,
            text_size: None,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::default(),
            wrapping: text::Wrapping::default(),
            letter_spacing: crate::core::Em::default(),
            font_features: Vec::new(),
            font: None,
            class: Theme::default(),
            last_status: None,
        }
    }

    /// Sets the label of the [`Radio`] button.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Sets the function that will be called when the [`Radio`] is clicked.
    /// It will receive the value of the [`Radio`] and must produce a
    /// `Message`.
    ///
    /// Unless `on_toggle` is called, the [`Radio`] will be disabled.
    pub fn on_toggle<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(V) -> Message,
    {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Sets the function that will be called when the [`Radio`] is clicked,
    /// if `Some`.
    ///
    /// If `None`, the [`Radio`] will be disabled.
    pub fn on_toggle_maybe<F>(mut self, f: Option<F>) -> Self
    where
        F: 'a + Fn(V) -> Message,
    {
        self.on_toggle = f.map(|f| Box::new(f) as _);
        self
    }

    /// Sets the size of the [`Radio`] button.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the width of the [`Radio`] button.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the spacing between the [`Radio`] button and the text.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0;
        self
    }

    /// Sets the text size of the [`Radio`] button.
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Radio`] button.
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Radio`] button.
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the [`text::Wrapping`] strategy of the [`Radio`] button.
    pub fn wrapping(mut self, wrapping: text::Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Sets the letter spacing of the [`Radio`] button.
    pub fn letter_spacing(
        mut self,
        letter_spacing: impl Into<crate::core::Em>,
    ) -> Self {
        self.letter_spacing = letter_spacing.into();
        self
    }

    /// Adds a single font [`Feature`](crate::core::font::Feature) to the [`Radio`] button.
    pub fn font_feature(
        mut self,
        feature: impl Into<crate::core::font::Feature>,
    ) -> Self {
        self.font_features.push(feature.into());
        self
    }

    /// Sets the font features of the [`Radio`] button.
    pub fn font_features(
        mut self,
        features: Vec<crate::core::font::Feature>,
    ) -> Self {
        self.font_features = features;
        self
    }

    /// Sets the text font of the [`Radio`] button.
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the style of the [`Radio`] button.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Radio`] button.
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

/// Internal state for the animated [`Radio`].
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
    for Radio<'_, V, Message, Theme, Renderer>
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
            if self.label.is_some() {
                self.spacing
            } else {
                0.0
            },
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
                            letter_spacing: self.letter_spacing,
                            font_features: self.font_features.clone(),
                            font_variations: Vec::new(),
                            weight: None,
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

        // While the selection is animating, interpolate background, border,
        // and dot colors between the off- and on-state styles so they fade
        // in sync with the dot, not snap when `is_selected` flips.
        let style = match state.now {
            Some(now) if state.animation.is_animating(now) => {
                let off = theme
                    .style(&self.class, current_status.with_selected(false));
                let on = theme
                    .style(&self.class, current_status.with_selected(true));

                // Only `Background::Color` is interpolable here; fall back
                // to transparent for gradients so the draw code stays
                // simple.
                let bg_color = |bg: Background| match bg {
                    Background::Color(c) => c,
                    _ => Color::TRANSPARENT,
                };

                let background =
                    Background::Color(state.animation.interpolate(
                        bg_color(off.background),
                        bg_color(on.background),
                        now,
                    ));

                let border_color = state.animation.interpolate(
                    off.border_color,
                    on.border_color,
                    now,
                );

                let dot_color = state.animation.interpolate(
                    off.dot_color,
                    on.dot_color,
                    now,
                );

                let target = if state.animation.value() { on } else { off };
                Style {
                    background,
                    border_color,
                    dot_color,
                    ..target
                }
            }
            _ => theme.style(&self.class, current_status),
        };

        {
            let layout = children.next().unwrap();
            let bounds = layout.bounds();

            let size = bounds.width;
            let dot_size = size / 2.0;

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: (size / 2.0).into(),
                        width: style.border_width,
                        color: style.border_color,
                    },
                    ..renderer::Quad::default()
                },
                style.background,
            );

            // Drive the dot from a 0..1 progress: alpha fades in and the
            // dot scales up about its center so it pops in rather than
            // snapping. When idle, progress collapses to 0 or 1.
            let progress = match state.now {
                Some(now) => state.animation.interpolate(0.0_f32, 1.0_f32, now),
                None => {
                    if self.is_selected {
                        1.0
                    } else {
                        0.0
                    }
                }
            };

            if progress > 0.0 {
                let scaled = dot_size * progress;
                let dot_color = Color {
                    a: style.dot_color.a * progress,
                    ..style.dot_color
                };

                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + (bounds.width - scaled) / 2.0,
                            y: bounds.y + (bounds.height - scaled) / 2.0,
                            width: scaled,
                            height: scaled,
                        },
                        border: border::rounded(scaled / 2.0),
                        ..renderer::Quad::default()
                    },
                    dot_color,
                );
            }
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
    From<Radio<'a, V, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    V: 'a + Clone,
    Message: 'a,
    Theme: 'a + Catalog,
    Renderer: 'a + text::Renderer,
{
    fn from(
        radio: Radio<'a, V, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(radio)
    }
}

/// The possible status of a [`Radio`] button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Radio`] button can be interacted with.
    Active {
        /// Indicates whether the [`Radio`] button is currently selected.
        is_selected: bool,
    },
    /// The [`Radio`] button is being hovered.
    Hovered {
        /// Indicates whether the [`Radio`] button is currently selected.
        is_selected: bool,
    },
    /// The [`Radio`] button cannot be interacted with.
    Disabled {
        /// Indicates whether the [`Radio`] button is currently selected.
        is_selected: bool,
    },
}

impl Status {
    /// Returns this [`Status`] with its `is_selected` field replaced.
    fn with_selected(self, is_selected: bool) -> Self {
        match self {
            Status::Active { .. } => Status::Active { is_selected },
            Status::Hovered { .. } => Status::Hovered { is_selected },
            Status::Disabled { .. } => Status::Disabled { is_selected },
        }
    }
}

/// The appearance of a radio button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the radio button.
    pub background: Background,
    /// The [`Color`] of the dot of the radio button.
    pub dot_color: Color,
    /// The border width of the radio button.
    pub border_width: f32,
    /// The border [`Color`] of the radio button.
    pub border_color: Color,
    /// The text [`Color`] of the radio button.
    pub text_color: Option<Color>,
}

/// The theme catalog of a [`Radio`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Radio`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a [`Radio`] button.
///
/// Toned down to match the rest of sweeten: an unselected button is a
/// hollow ring with a neutral border; selecting it fills the circle with
/// the primary accent and pops a small contrasting dot at its center —
/// the shadcn/radix "radio" look, and the direct analog of how the
/// [`checkbox`](crate::widget::checkbox) fills on check. Hovering raises
/// the background one step instead of flooding it with color. For the
/// classic iced styling, see [`legacy`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    match status {
        Status::Active { is_selected } => styled(
            palette.background.strong.color,
            palette.background.weak.color,
            palette.primary.strong,
            is_selected,
        ),
        Status::Hovered { is_selected } => styled(
            palette.background.strong.color,
            palette.background.strong.color,
            palette.primary.strong,
            is_selected,
        ),
        Status::Disabled { is_selected } => {
            let muted = palette::Pair {
                color: palette
                    .primary
                    .strong
                    .color
                    .mix(palette.background.base.color, 0.6),
                text: palette.primary.strong.text,
            };
            styled(
                palette.background.weak.color,
                palette.background.weaker.color,
                muted,
                is_selected,
            )
        }
    }
}

/// The classic iced style of a [`Radio`] button: a transparent circle
/// with a primary-colored border and dot; hovering tints the fill with
/// the primary weak color.
pub fn legacy(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let active = Style {
        background: Color::TRANSPARENT.into(),
        dot_color: palette.primary.strong.color,
        border_width: 1.0,
        border_color: palette.primary.strong.color,
        text_color: None,
    };

    match status {
        Status::Active { .. } => active,
        Status::Hovered { .. } => Style {
            dot_color: palette.primary.strong.color,
            background: palette.primary.weak.color.into(),
            ..active
        },
        Status::Disabled { .. } => {
            // Mute the accent toward the page background so a disabled
            // radio reads as inert while still hinting its selection.
            let muted = palette
                .primary
                .strong
                .color
                .mix(palette.background.base.color, 0.6);
            Style {
                dot_color: muted,
                border_color: muted,
                ..active
            }
        }
    }
}

/// Builds a [`Style`] from a neutral `border_color`, an unselected fill
/// `base`, and the `accent` [`Pair`](palette::Pair) used when selected.
///
/// Unselected renders as a hollow ring over `base`; selected fills the
/// circle with `accent.color` and draws the center dot in `accent.text`
/// so it reads against the fill. The dot color stays `accent.text` in
/// both states so it doesn't shift hue as the dot fades in.
fn styled(
    border_color: Color,
    base: Color,
    accent: palette::Pair,
    is_selected: bool,
) -> Style {
    let (background, border_color) = if is_selected {
        (accent.color, accent.color)
    } else {
        (base, border_color)
    };

    Style {
        background: Background::Color(background),
        dot_color: accent.text,
        border_width: 1.0,
        border_color,
        text_color: None,
    }
}
