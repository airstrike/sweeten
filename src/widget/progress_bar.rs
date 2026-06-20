//! Progress bars visualize the progression of an extended computer
//! operation, such as a download, file transfer, or installation.
//!
//! This is a sweetened version of [`iced`'s `progress_bar`] that owns its
//! own [`Animation<f32>`]: each time the parent re-renders with a new
//! `value`, the bar smoothly interpolates from its currently-displayed
//! value to the new target over 150ms using
//! `cubic-bezier(0.4, 0, 0.2, 1)` — Tailwind's `transition-all` default,
//! the same easing shadcn's `<Progress>` indicator inherits.
//!
//! An optional [`on_idle`] callback fires when the easing animation
//! settles at its target, useful for gating follow-up actions (dismissing
//! a splash, navigating, etc.) on the bar reaching a specific value.
//!
//! [`iced`'s `progress_bar`]: https://docs.iced.rs/iced/widget/progress_bar/
//! [`on_idle`]: ProgressBar::on_idle
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use sweeten::widget::progress_bar;
//!
//! struct State {
//!    progress: f32,
//! }
//!
//! enum Message {
//!     // ...
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     progress_bar(0.0..=100.0, state.progress).into()
//! }
//! ```
use crate::animation::cubic_bezier;
use crate::core::animation::Easing;
use crate::core::border::{self, Border};
use crate::core::layout;
use crate::core::mouse;
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    self, Animation, Background, Color, Element, Event, Layout, Length,
    Rectangle, Shell, Size, Theme, Widget,
};

use std::ops::RangeInclusive;
use std::time::Duration;

/// Duration of the value-eased transition. Matches Tailwind's
/// `transition-all` default — what shadcn's `<Progress>` uses on its
/// `<Indicator>` element.
const TRANSITION_DURATION: Duration = Duration::from_millis(150);

/// A bar that displays progress.
///
/// Whenever the [`value`] prop changes between renders, the widget eases
/// from the currently-displayed value to the new target over 150ms using
/// `cubic-bezier(0.4, 0, 0.2, 1)`.
///
/// [`value`]: ProgressBar
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use sweeten::widget::progress_bar;
///
/// struct State {
///    progress: f32,
/// }
///
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     progress_bar(0.0..=100.0, state.progress).into()
/// }
/// ```
pub struct ProgressBar<'a, Message, Theme = crate::Theme>
where
    Theme: Catalog,
{
    range: RangeInclusive<f32>,
    value: f32,
    length: Length,
    girth: Length,
    is_vertical: bool,
    class: Theme::Class<'a>,
    on_idle: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

impl<'a, Message, Theme> ProgressBar<'a, Message, Theme>
where
    Theme: Catalog,
{
    /// The default girth of a [`ProgressBar`].
    pub const DEFAULT_GIRTH: f32 = 30.0;

    /// Creates a new [`ProgressBar`].
    ///
    /// It expects:
    ///   * an inclusive range of possible values
    ///   * the current value of the [`ProgressBar`]
    pub fn new(range: RangeInclusive<f32>, value: f32) -> Self {
        ProgressBar {
            value: value.clamp(*range.start(), *range.end()),
            range,
            length: Length::Fill,
            girth: Length::from(Self::DEFAULT_GIRTH),
            is_vertical: false,
            class: Theme::default(),
            on_idle: None,
        }
    }

    /// Sets the width of the [`ProgressBar`].
    pub fn length(mut self, length: impl Into<Length>) -> Self {
        self.length = length.into();
        self
    }

    /// Sets the height of the [`ProgressBar`].
    pub fn girth(mut self, girth: impl Into<Length>) -> Self {
        self.girth = girth.into();
        self
    }

    /// Turns the [`ProgressBar`] into a vertical [`ProgressBar`].
    ///
    /// By default, a [`ProgressBar`] is horizontal.
    pub fn vertical(mut self) -> Self {
        self.is_vertical = true;
        self
    }

    /// Sets the style of the [`ProgressBar`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`ProgressBar`].
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Sets a callback to fire when the easing animation finishes. The
    /// callback receives the at-rest target value. Useful for gating
    /// follow-up actions (dismiss splash, navigate, etc.) on the bar
    /// reaching a particular value.
    #[must_use]
    pub fn on_idle(mut self, on_idle: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_idle = Some(Box::new(on_idle));
        self
    }

    fn width(&self) -> Length {
        if self.is_vertical {
            self.girth
        } else {
            self.length
        }
    }

    fn height(&self) -> Length {
        if self.is_vertical {
            self.length
        } else {
            self.girth
        }
    }
}

/// Per-instance animation state stored in the widget tree.
#[derive(Debug, Clone)]
struct State {
    /// The interpolated value being rendered right now.
    animated: Animation<f32>,
    /// Value `on_idle` was last fired for, or `None` while a transition
    /// is awaiting its acknowledging fire. Compared against `self.value`
    /// at idle each frame so the fire survives a missed frame between
    /// `diff` and the next `RedrawRequested`.
    settled_at: Option<f32>,
}

impl State {
    /// Seed the animation at `value` with the shadcn `transition-all`
    /// easing. Called from [`Widget::state`] so the first mount renders
    /// at the initial prop value rather than 0.0 — iced's `Tree::new`
    /// does NOT call `Widget::diff`, so any lazy-init via `diff()` would
    /// silently miss.
    fn seeded(value: f32) -> Self {
        Self {
            animated: Animation::new(value)
                .easing(Easing::Custom(|t| {
                    // cubic-bezier(0.4, 0, 0.2, 1) — shadcn's
                    // transition-all default.
                    cubic_bezier(0.4, 0.0, 0.2, 1.0, t)
                }))
                .duration(TRANSITION_DURATION),
            settled_at: Some(value),
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ProgressBar<'_, Message, Theme>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::seeded(self.value))
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width(),
            height: self.height(),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width(), self.height())
    }

    fn diff(&mut self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State>();
        if state.animated.value() != self.value {
            // Re-target without snapping: `Animation::go_mut` interpolates
            // from the currently displayed value to the new target.
            // `State::seeded` already initialised the at-rest value when
            // this widget first mounted, so we never get here with a
            // stale `0.0` baseline.
            state.animated.go_mut(self.value, Instant::now());
            state.settled_at = None;
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<State>();
            let now_animating = state.animated.is_animating(*now);
            if !now_animating && state.settled_at != Some(self.value) {
                if let Some(on_idle) = &self.on_idle {
                    shell.publish(on_idle(self.value));
                }
                state.settled_at = Some(self.value);
            }
            if now_animating {
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let (range_start, range_end) = self.range.clone().into_inner();

        let state = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let displayed = state
            .animated
            .interpolate_with(|v| v, now)
            .clamp(range_start, range_end);

        let length = if self.is_vertical {
            bounds.height
        } else {
            bounds.width
        };

        let active_progress_length = if range_start >= range_end {
            0.0
        } else {
            length * (displayed - range_start) / (range_end - range_start)
        };

        let style = theme.style(&self.class);

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle { ..bounds },
                border: style.border,
                ..renderer::Quad::default()
            },
            style.background,
        );

        if active_progress_length > 0.0 {
            let bounds = if self.is_vertical {
                Rectangle {
                    y: bounds.y + bounds.height - active_progress_length,
                    height: active_progress_length,
                    ..bounds
                }
            } else {
                Rectangle {
                    width: active_progress_length,
                    ..bounds
                }
            };

            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        color: Color::TRANSPARENT,
                        ..style.border
                    },
                    ..renderer::Quad::default()
                },
                style.bar,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<ProgressBar<'a, Message, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a + Catalog,
    Renderer: 'a + core::Renderer,
{
    fn from(
        progress_bar: ProgressBar<'a, Message, Theme>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(progress_bar)
    }
}

/// The appearance of a progress bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the progress bar.
    pub background: Background,
    /// The [`Background`] of the bar of the progress bar.
    pub bar: Background,
    /// The [`Border`] of the progress bar.
    pub border: Border,
}

/// The theme catalog of a [`ProgressBar`].
pub trait Catalog: Sized {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`ProgressBar`].
///
/// This is just a boxed closure: `Fn(&Theme) -> Style`.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// The primary style of a [`ProgressBar`].
pub fn primary(theme: &Theme) -> Style {
    let palette = theme.palette();

    styled(palette.background.strong.color, palette.primary.base.color)
}

/// The secondary style of a [`ProgressBar`].
pub fn secondary(theme: &Theme) -> Style {
    let palette = theme.palette();

    styled(
        palette.background.strong.color,
        palette.secondary.base.color,
    )
}

/// The success style of a [`ProgressBar`].
pub fn success(theme: &Theme) -> Style {
    let palette = theme.palette();

    styled(palette.background.strong.color, palette.success.base.color)
}

/// The warning style of a [`ProgressBar`].
pub fn warning(theme: &Theme) -> Style {
    let palette = theme.palette();

    styled(palette.background.strong.color, palette.warning.base.color)
}

/// The danger style of a [`ProgressBar`].
pub fn danger(theme: &Theme) -> Style {
    let palette = theme.palette();

    styled(palette.background.strong.color, palette.danger.base.color)
}

fn styled(
    background: impl Into<Background>,
    bar: impl Into<Background>,
) -> Style {
    Style {
        background: background.into(),
        bar: bar.into(),
        border: border::rounded(2),
    }
}
