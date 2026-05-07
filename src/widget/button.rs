//! Buttons allow your users to perform actions by pressing them.
//!
//! This is a sweetened version of `iced`'s [`button`] with additional support
//! for focus tracking:
//!
//! - [`Button::on_focus`] — Emit a message when the button gains focus
//! - [`Button::on_blur`] — Emit a message when the button loses focus
//!
//! The button is focusable when enabled (i.e., when [`Button::on_press`] is
//! set), and responds to `Enter` and `Space` keys when focused.
//!
//! [`button`]: https://docs.iced.rs/iced/widget/button/
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } }
//! # pub type State = ();
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! use iced::widget::button;
//!
//! #[derive(Clone)]
//! enum Message {
//!     ButtonPressed,
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     button("Press me!").on_press(Message::ButtonPressed).into()
//! }
//! ```
use std::time::Duration;

use crate::animation::cubic_bezier;
use crate::core::animation::Easing;
use crate::core::border::{self, Border};
use crate::core::keyboard;
use crate::core::keyboard::key;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::theme::palette;
use crate::core::time::Instant;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::Operation;
use crate::core::widget::operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Animation, Background, Color, Element, Event, Layout, Length, Padding,
    Rectangle, Shadow, Shell, Size, Theme, Vector, Widget,
};

/// A generic widget that produces a message when pressed.
///
/// This is a sweetened version of [`iced`'s `Button`] with support for
/// [`on_focus`] and [`on_blur`] messages. The button participates in
/// tab-based focus navigation when enabled.
///
/// [`iced`'s `Button`]: https://docs.iced.rs/iced/widget/button/struct.Button.html
/// [`on_focus`]: Button::on_focus
/// [`on_blur`]: Button::on_blur
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::button;
///
/// #[derive(Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     button("Press me!").on_press(Message::ButtonPressed).into()
/// }
/// ```
///
/// If a [`Button::on_press`] handler is not set, the resulting [`Button`] will
/// be disabled:
///
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::button;
///
/// #[derive(Clone)]
/// enum Message {
///     ButtonPressed,
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     button("I am disabled!").into()
/// }
/// ```
pub struct Button<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    content: Element<'a, Message, Theme, Renderer>,
    on_press: Option<OnPress<'a, Message>>,
    on_focus: Option<Message>,
    on_blur: Option<Message>,
    id: Option<widget::Id>,
    width: Length,
    height: Length,
    padding: Padding,
    clip: bool,
    class: Theme::Class<'a>,
    status: Option<Status>,
}

enum OnPress<'a, Message> {
    Direct(Message),
    Closure(Box<dyn Fn() -> Message + 'a>),
}

impl<Message: Clone> OnPress<'_, Message> {
    fn get(&self) -> Message {
        match self {
            OnPress::Direct(message) => message.clone(),
            OnPress::Closure(f) => f(),
        }
    }
}

impl<'a, Message, Theme, Renderer> Button<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    /// Creates a new [`Button`] with the given content.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let content = content.into();
        let size = content.as_widget().size_hint();

        Button {
            content,
            on_press: None,
            on_focus: None,
            on_blur: None,
            id: Some(widget::Id::unique()),
            width: size.width.fluid(),
            height: size.height.fluid(),
            padding: DEFAULT_PADDING,
            clip: false,
            class: Theme::default(),
            status: None,
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// Unless `on_press` is called, the [`Button`] will be disabled.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.on_press = Some(OnPress::Direct(on_press));
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    ///
    /// This is analogous to [`Button::on_press`], but using a closure to produce
    /// the message.
    ///
    /// This closure will only be called when the [`Button`] is actually pressed and,
    /// therefore, this method is useful to reduce overhead if creating the resulting
    /// message is slow.
    pub fn on_press_with(
        mut self,
        on_press: impl Fn() -> Message + 'a,
    ) -> Self {
        self.on_press = Some(OnPress::Closure(Box::new(on_press)));
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed,
    /// if `Some`.
    ///
    /// If `None`, the [`Button`] will be disabled.
    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.on_press = on_press.map(OnPress::Direct);
        self
    }

    /// Sets the message that should be produced when the [`Button`] gains
    /// focus.
    pub fn on_focus(mut self, on_focus: Message) -> Self {
        self.on_focus = Some(on_focus);
        self
    }

    /// Sets the message that should be produced when the [`Button`] loses
    /// focus.
    pub fn on_blur(mut self, on_blur: Message) -> Self {
        self.on_blur = Some(on_blur);
        self
    }

    /// Sets the [`widget::Id`] of the [`Button`].
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets whether the contents of the [`Button`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the style of the [`Button`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Button`].
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

#[derive(Debug, Clone)]
struct State {
    is_focused: bool,
    was_focused: bool,
    is_pressed: bool,
    /// Animated "is hovered" state. Transitions smoothly between
    /// [`Status::Active`] and [`Status::Hovered`] styles over the
    /// animation's duration using shadcn's standard cubic-bezier
    /// `(0.4, 0, 0.2, 1)` easing (Material's "standard" curve).
    hover: Animation<bool>,
    /// The most recent [`Instant`] seen from a
    /// [`window::Event::RedrawRequested`] event — latched in
    /// [`Widget::update`] and consumed in [`Widget::draw`] to evaluate
    /// the animation at the same time the frame was scheduled for.
    now: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_focused: false,
            was_focused: false,
            is_pressed: false,
            hover: Animation::new(false)
                .easing(Easing::Custom(|t| {
                    // cubic-bezier(0.4, 0, 0.2, 1) — shadcn's
                    // `transition-all` standard curve.
                    cubic_bezier(0.4, 0.0, 0.2, 1.0, t)
                }))
                .duration(Duration::from_millis(150)),
            now: None,
        }
    }
}

impl operation::Focusable for State {
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

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Button<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + crate::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::padded(
            limits,
            self.width,
            self.height,
            self.padding,
            |limits| {
                self.content.as_widget_mut().layout(
                    &mut tree.children[0],
                    renderer,
                    limits,
                )
            },
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.on_press.is_some() {
            let state = tree.state.downcast_mut::<State>();
            operation.focusable(self.id.as_ref(), layout.bounds(), state);
        }

        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout.children().next().unwrap(),
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor,
            renderer,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        // Detect focus changes from operations (e.g., Tab key)
        {
            let state = tree.state.downcast_mut::<State>();
            if state.is_focused != state.was_focused {
                if state.is_focused {
                    if let Some(on_focus) = &self.on_focus {
                        shell.publish(on_focus.clone());
                    }
                } else if let Some(on_blur) = &self.on_blur {
                    shell.publish(on_blur.clone());
                }
                state.was_focused = state.is_focused;
            }
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let state = tree.state.downcast_mut::<State>();
                let bounds = layout.bounds();

                if cursor.is_over(bounds) {
                    if self.on_press.is_some() {
                        state.is_pressed = true;
                        shell.capture_event();

                        if !state.is_focused {
                            state.is_focused = true;
                            state.was_focused = true;
                            if let Some(on_focus) = &self.on_focus {
                                shell.publish(on_focus.clone());
                            }
                        }
                    }
                } else if state.is_focused {
                    state.is_focused = false;
                    state.was_focused = false;
                    if let Some(on_blur) = &self.on_blur {
                        shell.publish(on_blur.clone());
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. }) => {
                if let Some(on_press) = &self.on_press {
                    let state = tree.state.downcast_mut::<State>();

                    if state.is_pressed {
                        state.is_pressed = false;

                        let bounds = layout.bounds();

                        if cursor.is_over(bounds) {
                            shell.publish(on_press.get());
                        }

                        shell.capture_event();
                    }
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => {
                let state = tree.state.downcast_mut::<State>();

                state.is_pressed = false;
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                let state = tree.state.downcast_mut::<State>();

                if state.is_focused
                    && let Some(on_press) = &self.on_press
                {
                    match key.as_ref() {
                        keyboard::Key::Named(key::Named::Enter)
                        | keyboard::Key::Named(key::Named::Space) => {
                            shell.publish(on_press.get());
                            shell.capture_event();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        let state = tree.state.downcast_mut::<State>();
        let is_hovered = cursor.is_over(layout.bounds());

        let current_status = if self.on_press.is_none() {
            Status::Disabled
        } else if is_hovered && state.is_pressed {
            Status::Pressed
        } else if state.is_focused {
            Status::Focused { is_hovered }
        } else if is_hovered {
            Status::Hovered
        } else {
            Status::Active
        };

        // Arm the hover animation only inside `RedrawRequested`:
        // the `Instant` passed to `go_mut` has to be the same one
        // `draw` later reads off `state.now`, or the interpolation
        // jitters.
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = Some(*now);
            self.status = Some(current_status);

            let should_hover = is_hovered && self.on_press.is_some();
            if should_hover != state.hover.value() {
                state.hover.go_mut(should_hover, *now);
            }
            if state.hover.is_animating(*now) {
                shell.request_redraw();
            }
        } else if self.status.is_some_and(|status| status != current_status) {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let state = tree.state.downcast_ref::<State>();
        let status = self.status.unwrap_or(Status::Disabled);

        // Trade-off: if the button becomes `Pressed` or `Focused`
        // while the hover animation is still running, it keeps
        // showing the interpolated Active↔Hovered color until the
        // animation settles, not the press/focus style.
        let style = match state.now {
            Some(now) if state.hover.is_animating(now) => {
                let active = theme.style(&self.class, Status::Active);
                let hovered = theme.style(&self.class, Status::Hovered);

                // Ghost/outline variants have no active background;
                // fall back to the hovered color with `alpha = 0.0`
                // so the fade is pure alpha across one color rather
                // than jumping channels.
                let bg_color = |style: &Style| {
                    style.background.and_then(|bg| match bg {
                        Background::Color(c) => Some(c),
                        _ => None,
                    })
                };
                let hovered_bg =
                    bg_color(&hovered).unwrap_or(Color::TRANSPARENT);
                let active_bg = bg_color(&active).unwrap_or(Color {
                    a: 0.0,
                    ..hovered_bg
                });

                let background = Background::Color(
                    state.hover.interpolate(active_bg, hovered_bg, now),
                );
                let text_color = state.hover.interpolate(
                    active.text_color,
                    hovered.text_color,
                    now,
                );

                // Border and shadow snap to the target. Interpolating
                // width or radius through sub-pixel values shimmers;
                // a snap inside the 150ms window is imperceptible.
                let toward_hovered = state.hover.value();
                Style {
                    background: Some(background),
                    text_color,
                    border: if toward_hovered {
                        hovered.border
                    } else {
                        active.border
                    },
                    shadow: if toward_hovered {
                        hovered.shadow
                    } else {
                        active.shadow
                    },
                    snap: active.snap,
                }
            }
            _ => theme.style(&self.class, status),
        };

        if style.background.is_some()
            || style.border.width > 0.0
            || style.shadow.color.a > 0.0
        {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
                    snap: style.snap,
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        let viewport = if self.clip {
            bounds.intersection(viewport).unwrap_or(*viewport)
        } else {
            *viewport
        };

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style.text_color,
            },
            content_layout,
            cursor,
            &viewport,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let is_mouse_over = cursor.is_over(layout.bounds());

        if is_mouse_over && self.on_press.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap(),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Button<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(button: Button<'a, Message, Theme, Renderer>) -> Self {
        Self::new(button)
    }
}

/// The default [`Padding`] of a [`Button`].
pub const DEFAULT_PADDING: Padding = Padding {
    top: 5.0,
    bottom: 5.0,
    right: 10.0,
    left: 10.0,
};

/// The possible status of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The [`Button`] can be pressed.
    Active,
    /// The [`Button`] can be pressed and it is being hovered.
    Hovered,
    /// The [`Button`] is being pressed.
    Pressed,
    /// The [`Button`] is focused via keyboard navigation.
    Focused {
        /// Whether the [`Button`] is hovered, while focused.
        is_hovered: bool,
    },
    /// The [`Button`] cannot be pressed.
    Disabled,
}

/// The style of a button.
///
/// If not specified with [`Button::style`]
/// the theme will provide the style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the button.
    pub background: Option<Background>,
    /// The text [`Color`] of the button.
    pub text_color: Color,
    /// The [`Border`] of the button.
    pub border: Border,
    /// The [`Shadow`] of the button.
    pub shadow: Shadow,
    /// Whether the button should be snapped to the pixel grid.
    pub snap: bool,
}

impl Style {
    /// Updates the [`Style`] with the given [`Background`].
    pub fn with_background(self, background: impl Into<Background>) -> Self {
        Self {
            background: Some(background.into()),
            ..self
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            text_color: Color::BLACK,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: cfg!(feature = "crisp"),
        }
    }
}

/// The theme catalog of a [`Button`].
///
/// All themes that can be used with [`Button`]
/// must implement this trait.
///
/// # Example
/// ```no_run
/// # use iced_widget::core::{Color, Background};
/// # use iced_widget::button::{Catalog, Status, Style};
/// # struct MyTheme;
/// #[derive(Debug, Default)]
/// pub enum ButtonClass {
///     #[default]
///     Primary,
///     Secondary,
///     Danger
/// }
///
/// impl Catalog for MyTheme {
///     type Class<'a> = ButtonClass;
///
///     fn default<'a>() -> Self::Class<'a> {
///         ButtonClass::default()
///     }
///
///     fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
///         let mut style = Style::default();
///
///         match class {
///             ButtonClass::Primary => {
///                 style.background = Some(Background::Color(Color::from_rgb(0.529, 0.808, 0.921)));
///             },
///             ButtonClass::Secondary => {
///                 style.background = Some(Background::Color(Color::WHITE));
///             },
///             ButtonClass::Danger => {
///                 style.background = Some(Background::Color(Color::from_rgb(0.941, 0.502, 0.502)));
///             },
///         }
///
///         style
///     }
/// }
/// ```
///
/// Although, in order to use [`Button::style`]
/// with `MyTheme`, [`Catalog::Class`] must implement
/// `From<StyleFn<'_, MyTheme>>`.
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a [`Button`].
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

// --- Interop with `iced_widget::button` ---

impl From<iced_widget::button::Style> for Style {
    fn from(s: iced_widget::button::Style) -> Self {
        Self {
            background: s.background,
            text_color: s.text_color,
            border: s.border,
            shadow: s.shadow,
            snap: s.snap,
        }
    }
}

impl From<Status> for iced_widget::button::Status {
    fn from(status: Status) -> Self {
        match status {
            Status::Active => iced_widget::button::Status::Active,
            Status::Hovered => iced_widget::button::Status::Hovered,
            Status::Pressed => iced_widget::button::Status::Pressed,
            Status::Focused { is_hovered: true } => {
                iced_widget::button::Status::Hovered
            }
            Status::Focused { is_hovered: false } => {
                iced_widget::button::Status::Active
            }
            Status::Disabled => iced_widget::button::Status::Disabled,
        }
    }
}

/// Wraps an [`iced_widget::button::StyleFn`] so it can be used as a
/// [`StyleFn`] for this button.
///
/// Our [`Status::Focused`] variant degrades to `Hovered` (if hovered) or
/// `Active` (if not) when calling the upstream style function.
pub fn from_iced_style<'a>(
    f: iced_widget::button::StyleFn<'a, Theme>,
) -> StyleFn<'a, Theme> {
    Box::new(move |theme, status| f(theme, status.into()).into())
}

/// A primary button; denoting a main action.
pub fn primary(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.primary.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.primary.strong.color)),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.primary.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.primary.strong.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A secondary button; denoting a complementary action.
pub fn secondary(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.secondary.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.secondary.strong.color)),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.secondary.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.secondary.strong.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A success button; denoting a good outcome.
pub fn success(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.success.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.success.strong.color)),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.success.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.success.strong.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A warning button; denoting a risky action.
pub fn warning(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.warning.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.warning.strong.color)),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.warning.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.warning.strong.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A danger button; denoting a destructive action.
pub fn danger(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.danger.base);

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            background: Some(Background::Color(palette.danger.strong.color)),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.danger.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.danger.strong.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A text button; useful for links.
pub fn text(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let base = Style {
        text_color: palette.background.base.text,
        ..Style::default()
    };

    match status {
        Status::Active | Status::Pressed => base,
        Status::Hovered => Style {
            text_color: palette.background.base.text.scale_alpha(0.8),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.primary.strong.color,
                width: 2.0,
                ..base.border
            },
            text_color: if is_hovered {
                palette.background.base.text.scale_alpha(0.8)
            } else {
                base.text_color
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A button using background shades.
pub fn background(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.background.base);

    match status {
        Status::Active => base,
        Status::Pressed => Style {
            background: Some(Background::Color(
                palette.background.strong.color,
            )),
            ..base
        },
        Status::Hovered => Style {
            background: Some(Background::Color(palette.background.weak.color)),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.primary.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.background.weak.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

/// A subtle button using weak background shades.
pub fn subtle(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();
    let base = styled(palette.background.weakest);

    match status {
        Status::Active => base,
        Status::Pressed => Style {
            background: Some(Background::Color(
                palette.background.strong.color,
            )),
            ..base
        },
        Status::Hovered => Style {
            background: Some(Background::Color(
                palette.background.weaker.color,
            )),
            ..base
        },
        Status::Focused { is_hovered } => Style {
            border: Border {
                color: palette.primary.strong.color,
                width: 2.0,
                ..base.border
            },
            background: if is_hovered {
                Some(Background::Color(palette.background.weaker.color))
            } else {
                base.background
            },
            ..base
        },
        Status::Disabled => disabled(base),
    }
}

fn styled(pair: palette::Pair) -> Style {
    Style {
        background: Some(Background::Color(pair.color)),
        text_color: pair.text,
        border: border::rounded(2),
        ..Style::default()
    }
}

fn disabled(style: Style) -> Style {
    Style {
        background: style
            .background
            .map(|background| background.scale_alpha(0.5)),
        text_color: style.text_color.scale_alpha(0.5),
        ..style
    }
}
