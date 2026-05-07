//! Checkboxes can be used to let users make binary choices.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::checkbox;
//!
//! struct State {
//!    is_checked: bool,
//! }
//!
//! enum Message {
//!     CheckboxToggled(bool),
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     checkbox(state.is_checked)
//!         .label("Toggle me!")
//!         .on_toggle(Message::CheckboxToggled)
//!         .into()
//! }
//!
//! fn update(state: &mut State, message: Message) {
//!     match message {
//!         Message::CheckboxToggled(is_checked) => {
//!             state.is_checked = is_checked;
//!         }
//!     }
//! }
//! ```
//! ![Checkbox drawn by `iced_wgpu`](https://github.com/iced-rs/iced/blob/7760618fb112074bc40b148944521f312152012a/docs/images/checkbox.png?raw=true)
use crate::animation::cubic_bezier;
use crate::core::alignment;
use crate::core::animation::Easing;
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
    Animation, Background, Border, Color, Element, Event, Layout, Length,
    Pixels, Rectangle, Shell, Size, Theme, Widget,
};

/// A box that can be checked.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use iced::widget::checkbox;
///
/// struct State {
///    is_checked: bool,
/// }
///
/// enum Message {
///     CheckboxToggled(bool),
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     checkbox(state.is_checked)
///         .label("Toggle me!")
///         .on_toggle(Message::CheckboxToggled)
///         .into()
/// }
///
/// fn update(state: &mut State, message: Message) {
///     match message {
///         Message::CheckboxToggled(is_checked) => {
///             state.is_checked = is_checked;
///         }
///     }
/// }
/// ```
/// ![Checkbox drawn by `iced_wgpu`](https://github.com/iced-rs/iced/blob/7760618fb112074bc40b148944521f312152012a/docs/images/checkbox.png?raw=true)
pub struct Checkbox<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Renderer: text::Renderer,
    Theme: Catalog,
{
    is_checked: bool,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
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
    icon: Icon<Renderer::Font>,
    class: Theme::Class<'a>,
    last_status: Option<Status>,
}

impl<'a, Message, Theme, Renderer> Checkbox<'a, Message, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: Catalog,
{
    /// The default size of a [`Checkbox`].
    const DEFAULT_SIZE: f32 = 16.0;

    /// Creates a new [`Checkbox`].
    ///
    /// It expects:
    ///   * a boolean describing whether the [`Checkbox`] is checked or not
    pub fn new(is_checked: bool) -> Self {
        Checkbox {
            is_checked,
            on_toggle: None,
            label: None,
            width: Length::Shrink,
            size: Self::DEFAULT_SIZE,
            spacing: Self::DEFAULT_SIZE / 2.0,
            text_size: None,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::default(),
            wrapping: text::Wrapping::default(),
            letter_spacing: crate::core::Em::default(),
            font_features: Vec::new(),
            font: None,
            icon: Icon {
                font: Renderer::ICON_FONT,
                code_point: Renderer::CHECKMARK_ICON,
                size: None,
                line_height: text::LineHeight::default(),
                shaping: text::Shaping::Basic,
            },
            class: Theme::default(),
            last_status: None,
        }
    }

    /// Sets the label of the [`Checkbox`].
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Sets the function that will be called when the [`Checkbox`] is toggled.
    /// It will receive the new state of the [`Checkbox`] and must produce a
    /// `Message`.
    ///
    /// Unless `on_toggle` is called, the [`Checkbox`] will be disabled.
    pub fn on_toggle<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(bool) -> Message,
    {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Sets the function that will be called when the [`Checkbox`] is toggled,
    /// if `Some`.
    ///
    /// If `None`, the checkbox will be disabled.
    pub fn on_toggle_maybe<F>(mut self, f: Option<F>) -> Self
    where
        F: Fn(bool) -> Message + 'a,
    {
        self.on_toggle = f.map(|f| Box::new(f) as _);
        self
    }

    /// Sets the size of the [`Checkbox`].
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into().0;
        self
    }

    /// Sets the width of the [`Checkbox`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the spacing between the [`Checkbox`] and the text.
    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.spacing = spacing.into().0;
        self
    }

    /// Sets the text size of the [`Checkbox`].
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Checkbox`].
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = line_height.into();
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Checkbox`].
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the [`text::Wrapping`] strategy of the [`Checkbox`].
    pub fn wrapping(mut self, wrapping: text::Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Sets the letter spacing of the [`Checkbox`].
    pub fn letter_spacing(
        mut self,
        letter_spacing: impl Into<crate::core::Em>,
    ) -> Self {
        self.letter_spacing = letter_spacing.into();
        self
    }

    /// Adds a single font [`Feature`](crate::core::font::Feature) to the [`Checkbox`].
    pub fn font_feature(
        mut self,
        feature: impl Into<crate::core::font::Feature>,
    ) -> Self {
        self.font_features.push(feature.into());
        self
    }

    /// Sets the font features of the [`Checkbox`].
    pub fn font_features(
        mut self,
        features: Vec<crate::core::font::Feature>,
    ) -> Self {
        self.font_features = features;
        self
    }

    /// Sets the [`Renderer::Font`] of the text of the [`Checkbox`].
    ///
    /// [`Renderer::Font`]: crate::core::text::Renderer
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Sets the [`Icon`] of the [`Checkbox`].
    pub fn icon(mut self, icon: Icon<Renderer::Font>) -> Self {
        self.icon = icon;
        self
    }

    /// Sets the style of the [`Checkbox`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Checkbox`].
    #[cfg(feature = "advanced")]
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

/// Internal state for the animated [`Checkbox`].
struct State<P: text::Paragraph> {
    paragraph: widget::text::State<P>,
    animation: Animation<bool>,
    now: Option<Instant>,
    last_is_checked: bool,
    /// Whether the most recent left-button / finger press landed inside
    /// the checkbox. Toggling fires on release, gated by this flag plus
    /// a fresh "still inside on release" bounds check — pressing
    /// outside and dragging in (or pressing in and dragging out before
    /// release) must not toggle.
    is_pressed: bool,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Checkbox<'_, Message, Theme, Renderer>
where
    Renderer: text::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph> {
            paragraph: widget::text::State::default(),
            animation: Animation::new(self.is_checked)
                .very_quick()
                // cubic-bezier(0, 0, 0.2, 1) — Tailwind v4's
                // `--ease-out`. Same curve the toggler uses.
                .easing(Easing::Custom(|t| {
                    cubic_bezier(0.0, 0.0, 0.2, 1.0, t)
                })),
            now: None,
            last_is_checked: self.is_checked,
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
        // enabled, so that in-flight animations complete even if the
        // checkbox becomes disabled.
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = Some(*now);

            if state.animation.is_animating(*now) {
                shell.request_redraw();
            }
        }

        if self.is_checked != state.last_is_checked {
            state.last_is_checked = self.is_checked;

            if let Some(now) = state.now {
                state.animation.go_mut(self.is_checked, now);
                shell.request_redraw();
            }
        }

        // Toggle on release, but only when *both* press and release
        // happened inside the bounds — pressing outside and dragging
        // in, or pressing inside and dragging out before release, must
        // not toggle. We track press-inside in `state.is_pressed` and
        // gate the publish on a fresh "still inside" bounds check.
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
                    shell.publish((on_toggle)(!self.is_checked));
                    shell.capture_event();
                }
            }
            _ => {}
        }

        let current_status = {
            let is_mouse_over = cursor.is_over(layout.bounds());
            let is_disabled = self.on_toggle.is_none();
            let is_checked = self.is_checked;

            if is_disabled {
                Status::Disabled { is_checked }
            } else if is_mouse_over {
                Status::Hovered { is_checked }
            } else {
                Status::Active { is_checked }
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
            is_checked: self.is_checked,
        });

        // While the checkbox is animating, interpolate background, border,
        // and icon colors between the off- and on-state styles so they fade
        // in sync with the checkmark, not snap when `is_checked` flips.
        let style = match state.now {
            Some(now) if state.animation.is_animating(now) => {
                let off = theme
                    .style(&self.class, current_status.with_checked(false));
                let on =
                    theme.style(&self.class, current_status.with_checked(true));

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

                // Snap border width and radius — interpolating them looks
                // jittery — but interpolate the color along with the fill.
                let target_border = if state.animation.value() {
                    on.border
                } else {
                    off.border
                };
                let border = Border {
                    color: state.animation.interpolate(
                        off.border.color,
                        on.border.color,
                        now,
                    ),
                    ..target_border
                };

                let icon_color = state.animation.interpolate(
                    off.icon_color,
                    on.icon_color,
                    now,
                );

                let target = if state.animation.value() { on } else { off };
                Style {
                    background,
                    border,
                    icon_color,
                    ..target
                }
            }
            _ => theme.style(&self.class, current_status),
        };

        {
            let layout = children.next().unwrap();
            let bounds = layout.bounds();

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    ..renderer::Quad::default()
                },
                style.background,
            );

            let Icon {
                font,
                code_point,
                size,
                line_height,
                shaping,
            } = &self.icon;
            let size = size.unwrap_or(Pixels(bounds.height * 0.7));

            // Drive the checkmark's appearance from a 0..1 progress: alpha
            // fades in and the glyph scales up so the check pops in rather
            // than snapping. When idle, progress collapses to 0 or 1.
            let progress = match state.now {
                Some(now) => state.animation.interpolate(0.0_f32, 1.0_f32, now),
                None => {
                    if self.is_checked {
                        1.0
                    } else {
                        0.0
                    }
                }
            };

            if progress > 0.0 {
                let icon_color = Color {
                    a: style.icon_color.a * progress,
                    ..style.icon_color
                };

                renderer.fill_text(
                    text::Text {
                        content: code_point.to_string(),
                        font: *font,
                        size: Pixels(size.0 * progress),
                        line_height: *line_height,
                        bounds: bounds.size(),
                        align_x: text::Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        shaping: *shaping,
                        wrapping: text::Wrapping::default(),
                        ellipsis: text::Ellipsis::default(),
                        letter_spacing: crate::core::Em::default(),
                        font_features: Vec::new(),
                        font_variations: Vec::new(),
                        weight: None,
                        hint_factor: None,
                    },
                    bounds.center(),
                    icon_color,
                    *viewport,
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

impl<'a, Message, Theme, Renderer> From<Checkbox<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a + Catalog,
    Renderer: 'a + text::Renderer,
{
    fn from(
        checkbox: Checkbox<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(checkbox)
    }
}

/// The icon in a [`Checkbox`].
#[derive(Debug, Clone, PartialEq)]
pub struct Icon<Font> {
    /// Font that will be used to display the `code_point`,
    pub font: Font,
    /// The unicode code point that will be used as the icon.
    pub code_point: char,
    /// Font size of the content.
    pub size: Option<Pixels>,
    /// The line height of the icon.
    pub line_height: text::LineHeight,
    /// The shaping strategy of the icon.
    pub shaping: text::Shaping,
}

/// The possible status of a [`Checkbox`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Checkbox`] can be interacted with.
    Active {
        /// Indicates if the [`Checkbox`] is currently checked.
        is_checked: bool,
    },
    /// The [`Checkbox`] can be interacted with and it is being hovered.
    Hovered {
        /// Indicates if the [`Checkbox`] is currently checked.
        is_checked: bool,
    },
    /// The [`Checkbox`] cannot be interacted with.
    Disabled {
        /// Indicates if the [`Checkbox`] is currently checked.
        is_checked: bool,
    },
}

impl Status {
    /// Returns this [`Status`] with its `is_checked` field replaced.
    fn with_checked(self, is_checked: bool) -> Self {
        match self {
            Status::Active { .. } => Status::Active { is_checked },
            Status::Hovered { .. } => Status::Hovered { is_checked },
            Status::Disabled { .. } => Status::Disabled { is_checked },
        }
    }
}

/// The style of a checkbox.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the checkbox.
    pub background: Background,
    /// The icon [`Color`] of the checkbox.
    pub icon_color: Color,
    /// The [`Border`] of the checkbox.
    pub border: Border,
    /// The text [`Color`] of the checkbox.
    pub text_color: Option<Color>,
}

/// The theme catalog of a [`Checkbox`].
pub trait Catalog: Sized {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Checkbox`].
///
/// This is just a boxed closure: `Fn(&Theme, Status) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// A primary checkbox; denoting a main toggle.
pub fn primary(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    match status {
        Status::Active { is_checked } => styled(
            palette.background.strong.color,
            palette.background.base,
            palette.primary.base.text,
            palette.primary.base,
            is_checked,
        ),
        Status::Hovered { is_checked } => styled(
            palette.background.strong.color,
            palette.background.weak,
            palette.primary.base.text,
            palette.primary.strong,
            is_checked,
        ),
        Status::Disabled { is_checked } => {
            let accent = weakest_pair(
                palette.primary.weak,
                palette.background.base.color,
            );
            styled(
                palette.background.weak.color,
                palette.background.weaker,
                accent.text,
                accent,
                is_checked,
            )
        }
    }
}

/// A secondary checkbox; denoting a complementary toggle.
pub fn secondary(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    // Sweetened: paint with `palette.secondary.*` (matching iced's
    // `button::secondary`) instead of upstream's `palette.background.*`,
    // which made `secondary-active` collide with `primary-disabled`.
    match status {
        Status::Active { is_checked } => styled(
            palette.background.strong.color,
            palette.background.base,
            palette.secondary.base.text,
            palette.secondary.base,
            is_checked,
        ),
        Status::Hovered { is_checked } => styled(
            palette.background.strong.color,
            palette.background.weak,
            palette.secondary.base.text,
            palette.secondary.strong,
            is_checked,
        ),
        Status::Disabled { is_checked } => {
            let accent = weakest_pair(
                palette.secondary.weak,
                palette.background.base.color,
            );
            styled(
                palette.background.weak.color,
                palette.background.weaker,
                accent.text,
                accent,
                is_checked,
            )
        }
    }
}

/// A success checkbox; denoting a positive toggle.
pub fn success(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    match status {
        Status::Active { is_checked } => styled(
            palette.background.weak.color,
            palette.background.base,
            palette.success.base.text,
            palette.success.base,
            is_checked,
        ),
        Status::Hovered { is_checked } => styled(
            palette.background.strong.color,
            palette.background.weak,
            palette.success.base.text,
            palette.success.strong,
            is_checked,
        ),
        Status::Disabled { is_checked } => {
            let accent = weakest_pair(
                palette.success.weak,
                palette.background.base.color,
            );
            styled(
                palette.background.weak.color,
                palette.background.weaker,
                accent.text,
                accent,
                is_checked,
            )
        }
    }
}

/// A monochrome checkbox; uses the theme's body text color as the
/// accent so the checked state reads as a filled black/white block
/// matching surrounding type. Pairs well with text-only buttons.
pub fn text(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    // Active accent: inverse of `palette.background.base` — fill in
    // body text color, page-bg color as the icon channel.
    let inverse = palette::Pair {
        color: palette.background.base.text,
        text: palette.background.base.color,
    };
    // Hover accent: deviate the body text color further from the
    // page bg (lighter in dark themes, darker in light themes) — the
    // monochrome counterpart of the colored variants' `.strong`
    // shift. Same `0.15` factor `Background::new` uses for `strong`.
    let hover = palette::Pair {
        color: palette::deviate(palette.background.base.text, 0.15),
        text: palette.background.base.color,
    };

    match status {
        // Unchecked border tracks the same neutral as the colored
        // variants (`background.strong.color`) so the empty box reads
        // at the same visual weight; the body text color shows up
        // only as the *fill* on check.
        Status::Active { is_checked } => styled(
            palette.background.strong.color,
            palette.background.base,
            inverse.text,
            inverse,
            is_checked,
        ),
        Status::Hovered { is_checked } => styled(
            palette.background.strong.color,
            palette.background.weak,
            hover.text,
            hover,
            is_checked,
        ),
        Status::Disabled { is_checked } => styled(
            palette.background.weak.color,
            palette.background.weakest,
            palette
                .background
                .base
                .text
                .mix(palette.background.base.color, 0.55),
            palette.background.weakest,
            is_checked,
        ),
    }
}

/// A danger checkbox; denoting a negative toggle.
pub fn danger(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    match status {
        Status::Active { is_checked } => styled(
            palette.background.strong.color,
            palette.background.base,
            palette.danger.base.text,
            palette.danger.base,
            is_checked,
        ),
        Status::Hovered { is_checked } => styled(
            palette.background.strong.color,
            palette.background.weak,
            palette.danger.base.text,
            palette.danger.strong,
            is_checked,
        ),
        Status::Disabled { is_checked } => {
            let accent = weakest_pair(
                palette.danger.weak,
                palette.background.base.color,
            );
            styled(
                palette.background.weak.color,
                palette.background.weaker,
                accent.text,
                accent,
                is_checked,
            )
        }
    }
}

/// Synthesize a "weakest" disabled accent. `Swatch` tops out at
/// `.weak` (60% variant + 40% bg, still saturated enough to read as
/// active in dark themes), so for disabled we mix `.weak` further
/// toward the page bg to produce a barely-tinted fill, then mute
/// the paired icon by mixing its text channel toward the bg too.
fn weakest_pair(weak: palette::Pair, bg: Color) -> palette::Pair {
    palette::Pair {
        color: weak.color.mix(bg, 0.7),
        text: weak.text.mix(bg, 0.55),
    }
}

fn styled(
    border_color: Color,
    base: palette::Pair,
    icon_color: Color,
    accent: palette::Pair,
    is_checked: bool,
) -> Style {
    let (background, border) = if is_checked {
        (accent, accent.color)
    } else {
        (base, border_color)
    };

    Style {
        background: Background::Color(background.color),
        icon_color,
        border: Border {
            radius: 2.0.into(),
            width: 1.0,
            color: border,
        },
        text_color: None,
    }
}
