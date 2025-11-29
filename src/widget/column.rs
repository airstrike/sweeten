//! Distribute content vertically.
//!
//! This is a sweetened version of `iced`'s [`Column`] with drag-and-drop
//! reordering support via [`Column::on_drag`].
//!
//! [`Column`]: https://docs.iced.rs/iced/widget/struct.Column.html
//!
//! # Example
//!
//! ```no_run
//! # pub type Element<'a, Message> = iced::Element<'a, Message>;
//! use sweeten::widget::column;
//! use sweeten::widget::drag::DragEvent;
//!
//! #[derive(Clone)]
//! enum Message {
//!     Reorder(DragEvent),
//! }
//!
//! fn view(items: &[String]) -> Element<'_, Message> {
//!     column(items.iter().map(|s| s.as_str().into()))
//!         .spacing(5)
//!         .on_drag(Message::Reorder)
//!         .into()
//! }
//! ```

use crate::core::alignment::{self, Alignment};
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::widget::{Operation, Tree, tree};
use crate::core::{
    Animation, Background, Border, Clipboard, Color, Element, Event, Length,
    Padding, Pixels, Point, Rectangle, Shell, Size, Transformation, Vector,
    Widget,
};

use super::drag::DragEvent;

const DRAG_DEADBAND_DISTANCE: f32 = 5.0;

/// A container that distributes its contents vertically.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::{button, column};
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     column![
///         "I am on top!",
///         button("I am in the center!"),
///         "I am below.",
///     ].into()
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct Column<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: Catalog,
{
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    max_width: f32,
    align: Alignment,
    clip: bool,
    deadband_zone: f32,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    on_drag: Option<Box<dyn Fn(DragEvent) -> Message + 'a>>,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> Column<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    /// Creates an empty [`Column`].
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    /// Creates a [`Column`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_vec(Vec::with_capacity(capacity))
    }

    /// Creates a [`Column`] with the given elements.
    pub fn with_children(
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let iterator = children.into_iter();

        Self::with_capacity(iterator.size_hint().0).extend(iterator)
    }

    /// Creates a [`Column`] from an already allocated [`Vec`].
    ///
    /// Keep in mind that the [`Column`] will not inspect the [`Vec`], which means
    /// it won't automatically adapt to the sizing strategy of its contents.
    ///
    /// If any of the children have a [`Length::Fill`] strategy, you will need to
    /// call [`Column::width`] or [`Column::height`] accordingly.
    pub fn from_vec(
        children: Vec<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            spacing: 0.0,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: f32::INFINITY,
            align: Alignment::Start,
            clip: false,
            deadband_zone: DRAG_DEADBAND_DISTANCE,
            children,
            class: Theme::default(),
            on_drag: None,
        }
    }

    /// Sets the vertical spacing _between_ elements.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets the [`Padding`] of the [`Column`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the [`Column`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Column`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the maximum width of the [`Column`].
    pub fn max_width(mut self, max_width: impl Into<Pixels>) -> Self {
        self.max_width = max_width.into().0;
        self
    }

    /// Sets the horizontal alignment of the contents of the [`Column`].
    pub fn align_x(mut self, align: impl Into<alignment::Horizontal>) -> Self {
        self.align = Alignment::from(align.into());
        self
    }

    /// Sets whether the contents of the [`Column`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the drag deadband zone of the [`Column`].
    ///
    /// This is the minimum distance in pixels that the cursor must move
    /// before a drag operation begins. Default is 5.0 pixels.
    pub fn deadband_zone(mut self, deadband_zone: f32) -> Self {
        self.deadband_zone = deadband_zone;
        self
    }

    /// Adds an element to the [`Column`].
    pub fn push(
        mut self,
        child: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let child = child.into();
        let child_size = child.as_widget().size_hint();

        if !child_size.is_void() {
            self.width = self.width.enclose(child_size.width);
            self.height = self.height.enclose(child_size.height);
            self.children.push(child);
        }

        self
    }

    /// Adds an element to the [`Column`], if `Some`.
    pub fn push_maybe(
        self,
        child: Option<impl Into<Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
        if let Some(child) = child {
            self.push(child)
        } else {
            self
        }
    }

    /// Sets the style of the [`Column`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Column`].
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Extends the [`Column`] with the given children.
    pub fn extend(
        self,
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        children.into_iter().fold(self, Self::push)
    }

    /// Sets a handler for drag events.
    ///
    /// When set, items in the [`Column`] can be dragged and reordered.
    /// The handler receives a [`DragEvent`] describing what happened.
    pub fn on_drag(
        mut self,
        on_drag: impl Fn(DragEvent) -> Message + 'a,
    ) -> Self {
        self.on_drag = Some(Box::new(on_drag));
        self
    }

    /// Computes the index where a dragged item should be dropped.
    fn compute_target_index(
        &self,
        cursor_position: Point,
        layout: Layout<'_>,
    ) -> usize {
        let bounds = layout.bounds();
        let cursor_y = cursor_position.y;

        if cursor_y < bounds.y {
            return 0;
        }

        for (i, child_layout) in layout.children().enumerate() {
            let bounds = child_layout.bounds();
            let y = bounds.y;
            let height = bounds.height;

            if cursor_y <= y + height {
                return i;
            }
        }

        self.children.len().saturating_sub(1)
    }
}

impl<Message, Theme, Renderer> Default for Column<'_, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message, Theme, Renderer: crate::core::Renderer>
    FromIterator<Element<'a, Message, Theme, Renderer>>
    for Column<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
{
    fn from_iter<
        T: IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    >(
        iter: T,
    ) -> Self {
        Self::with_children(iter)
    }
}

// Internal state for drag animations
#[derive(Debug, Clone)]
enum Action {
    Idle {
        now: Option<Instant>,
        animations: ItemAnimations,
    },
    Picking {
        index: usize,
        origin: Point,
        now: Instant,
        animations: ItemAnimations,
    },
    Dragging {
        index: usize,
        origin: Point,
        last_cursor: Point,
        now: Instant,
        animations: ItemAnimations,
    },
}

impl Default for Action {
    fn default() -> Self {
        Self::Idle {
            now: None,
            animations: ItemAnimations::default(),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct ItemAnimations {
    offsets: Vec<Animation<f32>>,
}

impl ItemAnimations {
    fn zero(&mut self) {
        for animation in &mut self.offsets {
            *animation = Animation::new(0.0);
        }
    }

    fn is_animating(&self, now: Instant) -> bool {
        self.offsets.iter().any(|anim| anim.is_animating(now))
    }

    fn with_capacity(&mut self, count: usize) {
        if self.offsets.len() < count {
            self.offsets.resize_with(count, || Animation::new(0.0));
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Column<'_, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Action>()
    }

    fn state(&self) -> tree::State {
        let mut animations = ItemAnimations::default();
        animations.with_capacity(self.children.len());

        tree::State::new(Action::Idle {
            now: Some(Instant::now()),
            animations,
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);

        let action = tree.state.downcast_mut::<Action>();

        match action {
            Action::Idle { animations, .. }
            | Action::Picking { animations, .. }
            | Action::Dragging { animations, .. } => {
                animations.with_capacity(self.children.len());
            }
        }
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
        let limits = limits.max_width(self.max_width);

        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            &limits,
            self.width,
            self.height,
            self.padding,
            self.spacing,
            self.align,
            &mut self.children,
            &mut tree.children,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let action = tree.state.downcast_mut::<Action>();

        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell,
                viewport,
            );
        }

        if shell.is_event_captured() {
            return;
        }

        match &event {
            Event::Window(crate::core::window::Event::RedrawRequested(now)) => {
                match action {
                    Action::Idle {
                        now: current_now,
                        animations,
                    } => {
                        *current_now = Some(*now);

                        if animations.is_animating(*now) {
                            shell.request_redraw();
                        }
                    }
                    Action::Picking {
                        now: current_now, ..
                    }
                    | Action::Dragging {
                        now: current_now, ..
                    } => {
                        *current_now = *now;
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if self.on_drag.is_some() {
                    if let Some(cursor_position) =
                        cursor.position_over(layout.bounds())
                    {
                        let animations = match action {
                            Action::Idle { animations, .. } => animations,
                            Action::Picking { animations, .. } => animations,
                            Action::Dragging { animations, .. } => animations,
                        };
                        animations.zero();

                        let index =
                            self.compute_target_index(cursor_position, layout);

                        *action = Action::Picking {
                            index,
                            origin: cursor_position,
                            now: Instant::now(),
                            animations: std::mem::take(animations),
                        };

                        shell.capture_event();
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => match action {
                Action::Picking {
                    index,
                    origin,
                    now,
                    animations,
                } => {
                    if let Some(cursor_position) = cursor.position() {
                        if cursor_position.distance(*origin)
                            > self.deadband_zone
                        {
                            let index = *index;
                            let origin = *origin;
                            let now = *now;

                            *action = Action::Dragging {
                                index,
                                origin,
                                last_cursor: cursor_position,
                                now,
                                animations: std::mem::take(animations),
                            };

                            shell.request_redraw();

                            if let Some(on_drag) = &self.on_drag {
                                shell.publish(on_drag(DragEvent::Picked {
                                    index,
                                }));
                            }

                            shell.capture_event();
                        }
                    }
                }
                Action::Dragging {
                    origin,
                    index,
                    now,
                    animations,
                    ..
                } => {
                    shell.request_redraw();

                    if let Some(cursor_position) = cursor.position() {
                        animations.with_capacity(self.children.len());

                        let target_index =
                            self.compute_target_index(cursor_position, layout);

                        let drag_height = if let Some(child_layout) =
                            layout.children().nth(*index)
                        {
                            child_layout.bounds().height + self.spacing
                        } else {
                            0.0
                        };

                        for i in 0..animations.offsets.len() {
                            if i == *index {
                                animations.offsets[i]
                                    .go_mut(1.0, Instant::now());
                                continue;
                            }

                            let target_offset = match target_index.cmp(index) {
                                std::cmp::Ordering::Less
                                    if (target_index..*index).contains(&i) =>
                                {
                                    drag_height
                                }
                                std::cmp::Ordering::Greater
                                    if (*index + 1..=target_index)
                                        .contains(&i) =>
                                {
                                    -drag_height
                                }
                                _ => 0.0,
                            };

                            animations.offsets[i]
                                .go_mut(target_offset, Instant::now());
                        }

                        let origin = *origin;
                        let index = *index;
                        let now = *now;

                        *action = Action::Dragging {
                            last_cursor: cursor_position,
                            origin,
                            index,
                            now,
                            animations: std::mem::take(animations),
                        };

                        shell.capture_event();
                    } else {
                        let index = *index;
                        let now = *now;

                        if let Some(on_drag) = &self.on_drag {
                            shell.publish(on_drag(DragEvent::Canceled {
                                index,
                            }));
                        }

                        *action = Action::Idle {
                            now: Some(now),
                            animations: std::mem::take(animations),
                        };
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                match action {
                    Action::Dragging {
                        index,
                        animations,
                        now,
                        ..
                    } => {
                        let current_now = *now;

                        animations.with_capacity(self.children.len());

                        if let Some(cursor_position) = cursor.position() {
                            let target_index = self
                                .compute_target_index(cursor_position, layout);

                            let drag_height = if let Some(child_layout) =
                                layout.children().nth(*index)
                            {
                                child_layout.bounds().height + self.spacing
                            } else {
                                0.0
                            };

                            for i in 0..animations.offsets.len() {
                                let target_offset =
                                    match target_index.cmp(index) {
                                        std::cmp::Ordering::Less
                                            if (target_index..*index)
                                                .contains(&i) =>
                                        {
                                            drag_height
                                        }
                                        std::cmp::Ordering::Greater
                                            if (*index + 1..=target_index)
                                                .contains(&i) =>
                                        {
                                            -drag_height
                                        }
                                        _ => 0.0,
                                    };

                                if i == *index {
                                    animations.offsets[i] =
                                        Animation::new(target_offset);
                                } else {
                                    animations.offsets[i]
                                        .go_mut(target_offset, Instant::now());
                                }
                            }

                            if let Some(on_drag) = &self.on_drag {
                                shell.publish(on_drag(DragEvent::Dropped {
                                    index: *index,
                                    target_index,
                                }));
                                shell.capture_event();
                            }
                        } else if let Some(on_drag) = &self.on_drag {
                            shell.publish(on_drag(DragEvent::Canceled {
                                index: *index,
                            }));
                            shell.capture_event();
                        }

                        *action = Action::Idle {
                            now: Some(current_now),
                            animations: std::mem::take(animations),
                        };
                    }
                    Action::Picking {
                        animations, now, ..
                    } => {
                        *action = Action::Idle {
                            now: Some(*now),
                            animations: std::mem::take(animations),
                        };
                    }
                    _ => {}
                }
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let action = tree.state.downcast_ref::<Action>();

        if let Action::Dragging { .. } = *action {
            return mouse::Interaction::Grabbing;
        }

        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child.as_widget().mouse_interaction(
                    state, layout, cursor, viewport, renderer,
                )
            })
            .max()
            .unwrap_or_default()
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
        let action = tree.state.downcast_ref::<Action>();
        let style = theme.style(&self.class);

        match action {
            Action::Dragging {
                index,
                last_cursor,
                origin,
                now,
                animations,
                ..
            } => {
                let child_count = self.children.len();

                let target_index = if cursor.position().is_some() {
                    let target_index =
                        self.compute_target_index(*last_cursor, layout);
                    target_index.min(child_count - 1)
                } else {
                    *index
                };

                let drag_bounds =
                    layout.children().nth(*index).unwrap().bounds();
                let drag_height = drag_bounds.height + self.spacing;

                for i in 0..child_count {
                    let child = &self.children[i];
                    let state = &tree.children[i];
                    let child_layout = layout.children().nth(i).unwrap();

                    if i == *index {
                        let scale_factor = 1.0
                            + (style.scale - 1.0)
                                * animations.offsets[i]
                                    .interpolate_with(|v| v, *now);

                        let scaling = Transformation::scale(scale_factor);
                        let translation = *last_cursor - *origin * scaling;

                        renderer.with_translation(translation, |renderer| {
                            renderer.with_transformation(scaling, |renderer| {
                                renderer.with_layer(
                                    child_layout.bounds(),
                                    |renderer| {
                                        child.as_widget().draw(
                                            state,
                                            renderer,
                                            theme,
                                            defaults,
                                            child_layout,
                                            cursor,
                                            viewport,
                                        );
                                    },
                                );
                            });
                        });
                    } else {
                        let base_offset = if i < animations.offsets.len() {
                            animations.offsets[i].interpolate_with(|v| v, *now)
                        } else {
                            0.0
                        };

                        let offset = if base_offset == 0.0 {
                            match target_index.cmp(index) {
                                std::cmp::Ordering::Less
                                    if i >= target_index && i < *index =>
                                {
                                    drag_height
                                }
                                std::cmp::Ordering::Greater
                                    if i > *index && i <= target_index =>
                                {
                                    -drag_height
                                }
                                _ => 0.0,
                            }
                        } else {
                            base_offset
                        };

                        let translation = Vector::new(0.0, offset);

                        renderer.with_translation(translation, |renderer| {
                            child.as_widget().draw(
                                state,
                                renderer,
                                theme,
                                defaults,
                                child_layout,
                                cursor,
                                viewport,
                            );

                            if offset != 0.0 {
                                let progress = (offset / drag_height).abs();

                                renderer.fill_quad(
                                    renderer::Quad {
                                        bounds: child_layout.bounds(),
                                        ..renderer::Quad::default()
                                    },
                                    style
                                        .moved_item_overlay
                                        .scale_alpha(progress),
                                );
                            }
                        });
                    }
                }

                let target_index =
                    self.compute_target_index(*last_cursor, layout);
                let is_moving_up = target_index < *index;

                let ghost_translation = layout
                    .children()
                    .enumerate()
                    .filter(|(i, _)| *i != *index)
                    .fold(0.0, |acc, (i, child_layout)| {
                        if i < animations.offsets.len() {
                            let offset = animations.offsets[i]
                                .interpolate_with(|v| v, *now);

                            if offset != 0.0 {
                                let height =
                                    child_layout.bounds().height + self.spacing;

                                if is_moving_up
                                    && i >= target_index
                                    && i < *index
                                {
                                    return acc - height;
                                } else if !is_moving_up
                                    && i > *index
                                    && i <= target_index
                                {
                                    return acc + height;
                                }
                            }
                        }
                        acc
                    });

                let ghost_vector = Vector::new(0.0, ghost_translation);

                renderer.with_translation(ghost_vector, |renderer| {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: drag_bounds,
                            border: style.ghost_border,
                            ..renderer::Quad::default()
                        },
                        style.ghost_background,
                    );
                });
            }
            Action::Idle {
                now: Some(now),
                animations,
            } => {
                for (i, child) in self.children.iter().enumerate() {
                    let state = &tree.children[i];
                    let child_layout = layout.children().nth(i).unwrap();

                    let offset = if i < animations.offsets.len() {
                        let is_animating =
                            animations.offsets[i].is_animating(*now);

                        if is_animating {
                            animations.offsets[i].interpolate_with(|v| v, *now)
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    let translation = Vector::new(0.0, offset);

                    renderer.with_translation(translation, |renderer| {
                        child.as_widget().draw(
                            state,
                            renderer,
                            theme,
                            defaults,
                            child_layout,
                            cursor,
                            viewport,
                        );

                        if offset != 0.0 {
                            let height =
                                child_layout.bounds().height + self.spacing;
                            let progress = (offset / height).abs();

                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: child_layout.bounds(),
                                    ..renderer::Quad::default()
                                },
                                style.moved_item_overlay.scale_alpha(progress),
                            );
                        }
                    });
                }
            }
            _ => {
                for ((child, state), layout) in self
                    .children
                    .iter()
                    .zip(&tree.children)
                    .zip(layout.children())
                {
                    child.as_widget().draw(
                        state, renderer, theme, defaults, layout, cursor,
                        viewport,
                    );
                }
            }
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
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Column<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(column: Column<'a, Message, Theme, Renderer>) -> Self {
        Self::new(column)
    }
}

/// The theme catalog of a [`Column`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// The appearance of a [`Column`] during drag operations.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// The scaling to apply to a picked element while it's being dragged.
    pub scale: f32,
    /// The color of the overlay on items that are moved around.
    pub moved_item_overlay: Color,
    /// The border of the dragged item's ghost.
    pub ghost_border: Border,
    /// The background of the dragged item's ghost.
    pub ghost_background: Background,
}

/// A styling function for a [`Column`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for crate::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// The default style for a [`Column`].
pub fn default(theme: &crate::Theme) -> Style {
    Style {
        scale: 1.05,
        moved_item_overlay: theme
            .extended_palette()
            .primary
            .base
            .color
            .scale_alpha(0.2),
        ghost_border: Border {
            width: 1.0,
            color: theme.extended_palette().secondary.base.color,
            radius: 0.0.into(),
        },
        ghost_background: theme
            .extended_palette()
            .secondary
            .base
            .color
            .scale_alpha(0.2)
            .into(),
    }
}
