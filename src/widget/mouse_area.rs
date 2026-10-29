//! A container for capturing mouse events.
//!
//! This is a sweetened version of `iced`'s [`MouseArea`] where all event
//! handlers receive the cursor position as a [`Point`] and the
//! [`keyboard::Modifiers`] held at the time of the event.
//!
//! [`MouseArea`]: https://docs.iced.rs/iced/widget/struct.MouseArea.html
//!
//! # Example
//! ```no_run
//! # pub type State = ();
//! # pub type Element<'a, Message> = iced::Element<'a, Message>;
//! use iced::keyboard;
//! use iced::Point;
//! use iced::widget::text;
//! use sweeten::widget::mouse_area;
//!
//! #[derive(Clone)]
//! enum Message {
//!     Clicked(Point, keyboard::Modifiers),
//! }
//!
//! fn view(state: &State) -> impl Widget<'_, Message> {
//!     mouse_area(text("Click me!"))
//!         .on_press(Message::Clicked)
//!         .into()
//! }
//! ```
use crate::core::keyboard;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::touch;
use crate::core::widget::Meta;
use crate::core::widget::{Operation, Tree, tree};
use crate::core::{
    Event, Layout, Length, Point, Rectangle, Shell, Size, Vector, Widget,
};

/// Emit messages on mouse events.
pub struct MouseArea<'a, Message, W = crate::Element<'a, Message>> {
    content: W,
    on_press: Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_release: Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_double_click:
        Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_right_press:
        Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_right_release:
        Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_middle_press:
        Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_middle_release:
        Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_scroll: Option<Box<dyn Fn(mouse::ScrollDelta) -> Message + 'a>>,
    on_enter: Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_move: Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    on_exit: Option<Box<dyn Fn(Point, keyboard::Modifiers) -> Message + 'a>>,
    interaction: Option<mouse::Interaction>,
}

impl<'a, Message, W> MouseArea<'a, Message, W> {
    /// Sets the message to emit on a left button press.
    ///
    /// The closure receives the click position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_press(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_press = Some(Box::new(f));
        self
    }

    /// Sets the message to emit on a left button press, if `Some`.
    ///
    /// The closure receives the click position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_press_maybe(
        mut self,
        f: Option<impl Fn(Point, keyboard::Modifiers) -> Message + 'a>,
    ) -> Self {
        self.on_press = f.map(|f| Box::new(f) as _);
        self
    }

    /// Sets the message to emit on a left button release.
    ///
    /// The closure receives the release position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_release(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_release = Some(Box::new(f));
        self
    }

    /// Sets the message to emit on a double click.
    ///
    /// The closure receives the click position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    ///
    /// If you use this with [`on_press`]/[`on_release`], those
    /// events will be emitted as normal.
    ///
    /// The event stream will be: on_press -> on_release -> on_press
    /// -> on_double_click -> on_release -> on_press ...
    ///
    /// [`on_press`]: Self::on_press
    /// [`on_release`]: Self::on_release
    #[must_use]
    pub fn on_double_click(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_double_click = Some(Box::new(f));
        self
    }

    /// Sets the message to emit on a right button press.
    ///
    /// The closure receives the click position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_right_press(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_right_press = Some(Box::new(f));
        self
    }

    /// Sets the message to emit on a right button release.
    ///
    /// The closure receives the release position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_right_release(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_right_release = Some(Box::new(f));
        self
    }

    /// Sets the message to emit on a middle button press.
    ///
    /// The closure receives the click position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_middle_press(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_middle_press = Some(Box::new(f));
        self
    }

    /// Sets the message to emit on a middle button release.
    ///
    /// The closure receives the release position as a [`Point`] and
    /// the [`keyboard::Modifiers`] held at the time of the event.
    #[must_use]
    pub fn on_middle_release(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_middle_release = Some(Box::new(f));
        self
    }

    /// Sets the message to emit when the scroll wheel is used.
    #[must_use]
    pub fn on_scroll(
        mut self,
        on_scroll: impl Fn(mouse::ScrollDelta) -> Message + 'a,
    ) -> Self {
        self.on_scroll = Some(Box::new(on_scroll));
        self
    }

    /// Sets the message to emit when the mouse enters the area.
    ///
    /// The closure receives the entry position as a [`Point`].
    #[must_use]
    pub fn on_enter(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_enter = Some(Box::new(f));
        self
    }

    /// Sets the message to emit when the mouse moves in the area.
    ///
    /// The closure receives the current position as a [`Point`].
    #[must_use]
    pub fn on_move(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_move = Some(Box::new(f));
        self
    }

    /// Sets the message to emit when the mouse exits the area.
    ///
    /// The closure receives the exit position as a [`Point`].
    #[must_use]
    pub fn on_exit(
        mut self,
        f: impl Fn(Point, keyboard::Modifiers) -> Message + 'a,
    ) -> Self {
        self.on_exit = Some(Box::new(f));
        self
    }

    /// The [`mouse::Interaction`] to use when hovering the area.
    #[must_use]
    pub fn interaction(mut self, interaction: mouse::Interaction) -> Self {
        self.interaction = Some(interaction);
        self
    }
}

/// Local state of the [`MouseArea`].
#[derive(Default)]
struct State {
    is_hovered: bool,
    bounds: Rectangle,
    cursor_position: Option<Point>,
    previous_click: Option<mouse::Click>,
    modifiers: keyboard::Modifiers,
}

impl<'a, Message, W> MouseArea<'a, Message, W> {
    /// Creates a [`MouseArea`] with the given content.
    pub fn new(content: W) -> Self {
        MouseArea {
            content: content.into(),
            on_press: None,
            on_release: None,
            on_double_click: None,
            on_right_press: None,
            on_right_release: None,
            on_middle_press: None,
            on_middle_release: None,
            on_scroll: None,
            on_enter: None,
            on_move: None,
            on_exit: None,
            interaction: None,
        }
    }
}

impl<Message, W> Meta for MouseArea<'_, Message, W> {}

impl<W, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for MouseArea<'_, Message, W>
where
    Renderer: renderer::Renderer,
    Message: Clone,
    W: Widget<Message, Theme, Renderer>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) {
        self.content.layout(&mut tree.children[0], renderer, limits);

        tree.size = tree.children[0].size;
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.operate(
            &mut tree.children[0],
            layout,
            viewport,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        update(self, tree, event, layout, cursor, shell);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let content_interaction = self.content.mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );

        match (self.interaction, content_interaction) {
            (Some(interaction), mouse::Interaction::None)
                if cursor.is_over(layout.bounds()) =>
            {
                interaction
            }
            _ => content_interaction,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &renderer::Style,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
            window,
        )
    }
}

/// Processes the given [`Event`] and updates the [`State`] of an [`MouseArea`]
/// accordingly.
fn update<Message: Clone, W>(
    widget: &mut MouseArea<'_, Message, W>,
    tree: &mut Tree,
    event: &Event,
    layout: Layout,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
) {
    let state: &mut State = tree.state.downcast_mut();

    if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event
    {
        state.modifiers = *modifiers;
    }

    let cursor_position = cursor.position();
    let bounds = layout.bounds();

    if state.cursor_position != cursor_position || state.bounds != bounds {
        let was_hovered = state.is_hovered;

        state.is_hovered = cursor.is_over(layout.bounds());
        state.cursor_position = cursor_position;
        state.bounds = bounds;

        if let Some(position) = cursor.position_in(layout.bounds()) {
            match (
                widget.on_enter.as_ref(),
                widget.on_move.as_ref(),
                widget.on_exit.as_ref(),
            ) {
                (Some(on_enter), _, _) if state.is_hovered && !was_hovered => {
                    shell.publish(on_enter(position, state.modifiers));
                }
                (_, Some(on_move), _) if state.is_hovered => {
                    shell.publish(on_move(position, state.modifiers));
                }
                (_, _, Some(on_exit)) if !state.is_hovered && was_hovered => {
                    shell.publish(on_exit(position, state.modifiers));
                }
                _ => {}
            }
        }
    }

    if !cursor.is_over(layout.bounds()) {
        return;
    }

    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            if let Some(on_press) = widget.on_press.as_ref()
                && let Some(position) = cursor.position_in(layout.bounds())
            {
                shell.publish(on_press(position, state.modifiers));
                shell.capture_event();
            }

            if let Some(position) = cursor.position_in(layout.bounds())
                && let Some(on_double_click) = widget.on_double_click.as_ref()
            {
                let new_click = mouse::Click::new(
                    position,
                    mouse::Button::Left,
                    state.previous_click,
                );

                if new_click.kind() == mouse::click::Kind::Double {
                    shell.publish(on_double_click(position, state.modifiers));
                }

                state.previous_click = Some(new_click);

                // Even if this is not a double click, but the press is nevertheless
                // processed by us and should not be popup to parent widgets.
                shell.capture_event();
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerLifted { .. }) => {
            if let Some(on_release) = widget.on_release.as_ref()
                && let Some(position) = cursor.position_in(layout.bounds())
            {
                shell.publish(on_release(position, state.modifiers));
            }
        }
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
            if let Some(on_right_press) = widget.on_right_press.as_ref()
                && let Some(position) = cursor.position_in(layout.bounds())
            {
                shell.publish(on_right_press(position, state.modifiers));
                shell.capture_event();
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
            if let Some(on_right_release) = widget.on_right_release.as_ref()
                && let Some(position) = cursor.position_in(layout.bounds())
            {
                shell.publish(on_right_release(position, state.modifiers));
            }
        }
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
            if let Some(on_middle_press) = widget.on_middle_press.as_ref()
                && let Some(position) = cursor.position_in(layout.bounds())
            {
                shell.publish(on_middle_press(position, state.modifiers));
                shell.capture_event();
            }
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
            if let Some(on_middle_release) = widget.on_middle_release.as_ref()
                && let Some(position) = cursor.position_in(layout.bounds())
            {
                shell.publish(on_middle_release(position, state.modifiers));
            }
        }
        Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
            if let Some(on_scroll) = widget.on_scroll.as_ref() {
                shell.publish(on_scroll(*delta));
                shell.capture_event();
            }
        }
        _ => {}
    }
}
