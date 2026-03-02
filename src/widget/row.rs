//! Distribute content horizontally.
//!
//! This is a sweetened version of `iced`'s [`Row`] with drag-and-drop
//! reordering support via [`Row::on_drag`].
//!
//! [`Row`]: https://docs.iced.rs/iced/widget/struct.Row.html
//!
//! # Example
//!
//! ```no_run
//! # pub type Element<'a, Message> = iced::Element<'a, Message>;
//! use sweeten::widget::row;
//! use sweeten::widget::drag::DragEvent;
//!
//! #[derive(Clone)]
//! enum Message {
//!     Reorder(DragEvent),
//! }
//!
//! fn view(items: &[String]) -> Element<'_, Message> {
//!     row(items.iter().map(|s| s.as_str().into()))
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
use super::operation::position;

const DRAG_DEADBAND_DISTANCE: f32 = 5.0;

/// A container that distributes its contents horizontally.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } }
/// # pub type State = ();
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// use iced::widget::{button, row};
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     // ...
/// }
///
/// fn view(state: &State) -> Element<'_, Message> {
///     row![
///         "I am to the left!",
///         button("I am in the middle!"),
///         "I am to the right!",
///     ].into()
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct Row<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: Catalog,
{
    id: Option<position::Id>,
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align: Alignment,
    clip: bool,
    deadband_zone: f32,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    on_drag: Option<Box<dyn Fn(DragEvent) -> Message + 'a>>,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> Row<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    /// Creates an empty [`Row`].
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    /// Creates a [`Row`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_vec(Vec::with_capacity(capacity))
    }

    /// Creates a [`Row`] with the given elements.
    pub fn with_children(
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let iterator = children.into_iter();

        Self::with_capacity(iterator.size_hint().0).extend(iterator)
    }

    /// Creates a [`Row`] from an already allocated [`Vec`].
    ///
    /// Keep in mind that the [`Row`] will not inspect the [`Vec`], which means
    /// it won't automatically adapt to the sizing strategy of its contents.
    ///
    /// If any of the children have a [`Length::Fill`] strategy, you will need to
    /// call [`Row::width`] or [`Row::height`] accordingly.
    pub fn from_vec(
        children: Vec<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            id: None,
            spacing: 0.0,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align: Alignment::Start,
            clip: false,
            deadband_zone: DRAG_DEADBAND_DISTANCE,
            children,
            class: Theme::default(),
            on_drag: None,
        }
    }

    /// Sets the horizontal spacing _between_ elements.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets the [`Padding`] of the [`Row`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the [`Row`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Row`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the vertical alignment of the contents of the [`Row`] .
    pub fn align_y(mut self, align: impl Into<alignment::Vertical>) -> Self {
        self.align = Alignment::from(align.into());
        self
    }

    /// Sets whether the contents of the [`Row`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the [`Id`](position::Id) of the [`Row`] for position tracking.
    ///
    /// When set, the [`Row`] tracks the layout bounds of each child widget.
    /// Use [`position::find_position`] to query a child's position by index.
    pub fn id(mut self, id: position::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the drag deadband zone of the [`Row`].
    ///
    /// This is the minimum distance in pixels that the cursor must move
    /// before a drag operation begins. Default is 5.0 pixels.
    pub fn deadband_zone(mut self, deadband_zone: f32) -> Self {
        self.deadband_zone = deadband_zone;
        self
    }

    /// Adds an [`Element`] to the [`Row`].
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

    /// Adds an element to the [`Row`], if `Some`.
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

    /// Sets the style of the [`Row`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Row`].
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Extends the [`Row`] with the given children.
    pub fn extend(
        self,
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        children.into_iter().fold(self, Self::push)
    }

    /// Turns the [`Row`] into a [`Wrapping`] row.
    ///
    /// The original alignment of the [`Row`] is preserved per row wrapped.
    pub fn wrap(self) -> Wrapping<'a, Message, Theme, Renderer> {
        Wrapping {
            row: self,
            vertical_spacing: None,
            align_x: alignment::Horizontal::Left,
        }
    }

    /// Sets a handler for drag events.
    ///
    /// When set, items in the [`Row`] can be dragged and reordered.
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
        let mut closest_index = 0;
        let mut closest_dist = f32::INFINITY;

        for (i, child_layout) in layout.children().enumerate() {
            let bounds = child_layout.bounds();
            let center = bounds.center();
            let dist = cursor_position.distance(center);

            if dist < closest_dist {
                closest_dist = dist;
                closest_index = i;
            }
        }

        closest_index
    }
}

impl<Message, Theme, Renderer> Default for Row<'_, Message, Theme, Renderer>
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
    for Row<'a, Message, Theme, Renderer>
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

#[derive(Debug, Clone)]
struct ItemAnimations {
    offsets_x: Vec<Animation<f32>>,
    offsets_y: Vec<Animation<f32>>,
    wrap_extent: f32,
    cross_spacing: f32,
}

impl Default for ItemAnimations {
    fn default() -> Self {
        Self {
            offsets_x: Vec::new(),
            offsets_y: Vec::new(),
            wrap_extent: f32::INFINITY,
            cross_spacing: 0.0,
        }
    }
}

impl ItemAnimations {
    fn zero(&mut self) {
        for animation in &mut self.offsets_x {
            *animation = Animation::new(0.0);
        }
        for animation in &mut self.offsets_y {
            *animation = Animation::new(0.0);
        }
    }

    fn is_animating(&self, now: Instant) -> bool {
        self.offsets_x.iter().any(|anim| anim.is_animating(now))
            || self.offsets_y.iter().any(|anim| anim.is_animating(now))
    }

    fn with_capacity(&mut self, count: usize) {
        if self.offsets_x.len() < count {
            self.offsets_x.resize_with(count, || Animation::new(0.0));
        }
        if self.offsets_y.len() < count {
            self.offsets_y.resize_with(count, || Animation::new(0.0));
        }
    }
}

#[derive(Default)]
struct WidgetState {
    action: Action,
    positions: position::State,
}

/// Simulate the layout that would result from reordering items, returning
/// per-item offset vectors (simulated position - current position).
///
/// For Row: primary axis is X (left-to-right), cross axis is Y (wrapping into rows).
#[allow(clippy::too_many_arguments)]
fn compute_reorder_offsets(
    child_bounds: &[Rectangle],
    source: usize,
    target: usize,
    spacing: f32,
    cross_spacing: f32,
    wrap_extent: f32,
    start: Point,
    align: Alignment,
) -> Vec<Vector> {
    let n = child_bounds.len();
    if n == 0 {
        return Vec::new();
    }

    // Build reordered index sequence
    let mut order: Vec<usize> = (0..n).collect();
    let removed = order.remove(source);
    let insert_at = if target > source {
        (target).min(order.len())
    } else {
        target
    };
    order.insert(insert_at, removed);

    // First pass: simulate wrapping layout, track row boundaries
    let mut simulated_pos = vec![Vector::ZERO; n];
    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;
    let mut row_height: f32 = 0.0;
    let mut row_start: usize = 0;

    struct RowInfo {
        start: usize,
        end: usize,
        height: f32,
    }
    let mut rows: Vec<RowInfo> = Vec::new();

    for (seq, &idx) in order.iter().enumerate() {
        let w = child_bounds[idx].width;
        let h = child_bounds[idx].height;

        if x > 0.0 && x + w > wrap_extent {
            rows.push(RowInfo {
                start: row_start,
                end: seq,
                height: row_height,
            });
            y += row_height + cross_spacing;
            x = 0.0;
            row_height = 0.0;
            row_start = seq;
        }

        simulated_pos[idx] = Vector::new(x + start.x, y + start.y);
        row_height = row_height.max(h);
        x += w + spacing;
    }

    // Final row
    rows.push(RowInfo {
        start: row_start,
        end: n,
        height: row_height,
    });

    // Second pass: apply cross-axis alignment (vertical for Row)
    let align_factor = match align {
        Alignment::Start => 0.0,
        Alignment::Center => 2.0,
        Alignment::End => 1.0,
    };

    if align_factor != 0.0 {
        for row_info in &rows {
            for &idx in &order[row_info.start..row_info.end] {
                let h = child_bounds[idx].height;
                simulated_pos[idx].y += (row_info.height - h) / align_factor;
            }
        }
    }

    // Compute offsets: simulated - current
    (0..n)
        .map(|i| {
            simulated_pos[i] - (child_bounds[i].position() - Point::ORIGIN)
        })
        .collect()
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Row<'_, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<WidgetState>()
    }

    fn state(&self) -> tree::State {
        let mut animations = ItemAnimations::default();
        animations.with_capacity(self.children.len());

        tree::State::new(WidgetState {
            action: Action::Idle {
                now: Some(Instant::now()),
                animations,
            },
            positions: position::State::default(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);

        let action = &mut tree.state.downcast_mut::<WidgetState>().action;

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
        let action = &mut tree.state.downcast_mut::<WidgetState>().action;
        match action {
            Action::Idle { animations, .. }
            | Action::Picking { animations, .. }
            | Action::Dragging { animations, .. } => {
                animations.wrap_extent = f32::INFINITY;
                animations.cross_spacing = self.spacing;
            }
        }

        let node = layout::flex::resolve(
            layout::flex::Axis::Horizontal,
            renderer,
            limits,
            self.width,
            self.height,
            self.padding,
            self.spacing,
            self.align,
            &mut self.children,
            &mut tree.children,
        );

        if self.id.is_some() {
            let state = tree.state.downcast_mut::<WidgetState>();
            state.positions.clear();
            for (i, child) in node.children().iter().enumerate() {
                state.positions.set(i, child.bounds());
            }
        }

        node
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let id = self.id.as_ref().map(|id| &id.0);

        operation.container(id, layout.bounds());
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

        if id.is_some() {
            let state = tree.state.downcast_mut::<WidgetState>();
            operation.custom(id, layout.bounds(), &mut state.positions);
        }
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
        let action = &mut tree.state.downcast_mut::<WidgetState>().action;

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
                if self.on_drag.is_some()
                    && let Some(cursor_position) =
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
            Event::Mouse(mouse::Event::CursorMoved { .. }) => match action {
                Action::Picking {
                    index,
                    origin,
                    now,
                    animations,
                } => {
                    if let Some(cursor_position) = cursor.position()
                        && cursor_position.distance(*origin)
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
                            shell.publish(on_drag(DragEvent::Picked { index }));
                        }

                        shell.capture_event();
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

                        let child_bounds: Vec<Rectangle> =
                            layout.children().map(|l| l.bounds()).collect();

                        let start = layout.bounds().position()
                            + Vector::new(self.padding.left, self.padding.top);

                        let offsets = compute_reorder_offsets(
                            &child_bounds,
                            *index,
                            target_index,
                            self.spacing,
                            animations.cross_spacing,
                            animations.wrap_extent,
                            start,
                            self.align,
                        );

                        let now_instant = Instant::now();
                        let count =
                            child_bounds.len().min(animations.offsets_x.len());
                        for (i, offset) in
                            offsets.iter().enumerate().take(count)
                        {
                            if i == *index {
                                animations.offsets_x[i]
                                    .go_mut(1.0, now_instant);
                                animations.offsets_y[i]
                                    .go_mut(1.0, now_instant);
                            } else {
                                animations.offsets_x[i]
                                    .go_mut(offset.x, now_instant);
                                animations.offsets_y[i]
                                    .go_mut(offset.y, now_instant);
                            }
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

                            let child_bounds: Vec<Rectangle> =
                                layout.children().map(|l| l.bounds()).collect();

                            let start = layout.bounds().position()
                                + Vector::new(
                                    self.padding.left,
                                    self.padding.top,
                                );

                            let offsets = compute_reorder_offsets(
                                &child_bounds,
                                *index,
                                target_index,
                                self.spacing,
                                animations.cross_spacing,
                                animations.wrap_extent,
                                start,
                                self.align,
                            );

                            let now_instant = Instant::now();
                            let count = child_bounds
                                .len()
                                .min(animations.offsets_x.len());
                            for (i, offset) in
                                offsets.iter().enumerate().take(count)
                            {
                                if i == *index {
                                    animations.offsets_x[i] =
                                        Animation::new(offset.x);
                                    animations.offsets_y[i] =
                                        Animation::new(offset.y);
                                } else {
                                    animations.offsets_x[i]
                                        .go_mut(offset.x, now_instant);
                                    animations.offsets_y[i]
                                        .go_mut(offset.y, now_instant);
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
        let action = &tree.state.downcast_ref::<WidgetState>().action;

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
        let action = &tree.state.downcast_ref::<WidgetState>().action;
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

                let child_bounds: Vec<Rectangle> =
                    layout.children().map(|l| l.bounds()).collect();

                let start = layout.bounds().position()
                    + Vector::new(self.padding.left, self.padding.top);

                let offsets = compute_reorder_offsets(
                    &child_bounds,
                    *index,
                    target_index,
                    self.spacing,
                    animations.cross_spacing,
                    animations.wrap_extent,
                    start,
                    self.align,
                );

                for i in 0..child_count {
                    let child = &self.children[i];
                    let state = &tree.children[i];
                    let child_layout = layout.children().nth(i).unwrap();

                    if i == *index {
                        let scale_factor = 1.0
                            + (style.scale - 1.0)
                                * animations.offsets_x[i]
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
                        let offset_x = if i < animations.offsets_x.len() {
                            let v = animations.offsets_x[i]
                                .interpolate_with(|v| v, *now);
                            if v == 0.0 { offsets[i].x } else { v }
                        } else {
                            offsets[i].x
                        };
                        let offset_y = if i < animations.offsets_y.len() {
                            let v = animations.offsets_y[i]
                                .interpolate_with(|v| v, *now);
                            if v == 0.0 { offsets[i].y } else { v }
                        } else {
                            offsets[i].y
                        };

                        let translation = Vector::new(offset_x, offset_y);

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

                            if offset_x != 0.0 || offset_y != 0.0 {
                                let magnitude = (offset_x * offset_x
                                    + offset_y * offset_y)
                                    .sqrt();
                                let item_extent =
                                    child_bounds[i].width + self.spacing;
                                let progress =
                                    (magnitude / item_extent).min(1.0);

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

                let ghost_vector = offsets[*index];

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

                    let offset_x = if i < animations.offsets_x.len() {
                        let is_animating =
                            animations.offsets_x[i].is_animating(*now);
                        if is_animating {
                            animations.offsets_x[i]
                                .interpolate_with(|v| v, *now)
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    let offset_y = if i < animations.offsets_y.len() {
                        let is_animating =
                            animations.offsets_y[i].is_animating(*now);
                        if is_animating {
                            animations.offsets_y[i]
                                .interpolate_with(|v| v, *now)
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    let translation = Vector::new(offset_x, offset_y);

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

                        if offset_x != 0.0 || offset_y != 0.0 {
                            let magnitude = (offset_x * offset_x
                                + offset_y * offset_y)
                                .sqrt();
                            let size =
                                child_layout.bounds().width + self.spacing;
                            let progress = (magnitude / size).min(1.0);

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

impl<'a, Message, Theme, Renderer> From<Row<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(row: Row<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}

/// A [`Row`] that wraps its contents.
///
/// Create a [`Row`] first, and then call [`Row::wrap`] to
/// obtain a [`Row`] that wraps its contents.
///
/// The original alignment of the [`Row`] is preserved per row wrapped.
#[allow(missing_debug_implementations)]
pub struct Wrapping<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
{
    row: Row<'a, Message, Theme, Renderer>,
    vertical_spacing: Option<f32>,
    align_x: alignment::Horizontal,
}

impl<Message, Theme, Renderer> Wrapping<'_, Message, Theme, Renderer>
where
    Theme: Catalog,
{
    /// Sets the vertical spacing _between_ lines.
    pub fn vertical_spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.vertical_spacing = Some(amount.into().0);
        self
    }

    /// Sets the horizontal alignment of the wrapping [`Row`].
    pub fn align_x(
        mut self,
        align_x: impl Into<alignment::Horizontal>,
    ) -> Self {
        self.align_x = align_x.into();
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Wrapping<'_, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        self.row.tag()
    }

    fn state(&self) -> tree::State {
        self.row.state()
    }

    fn children(&self) -> Vec<Tree> {
        self.row.children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.row.diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.row.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits
            .width(self.row.width)
            .height(self.row.height)
            .shrink(self.row.padding);

        let child_limits = limits.loose();
        let spacing = self.row.spacing;
        let vertical_spacing = self.vertical_spacing.unwrap_or(spacing);
        let max_width = limits.max().width;

        let action = &mut tree.state.downcast_mut::<WidgetState>().action;
        match action {
            Action::Idle { animations, .. }
            | Action::Picking { animations, .. }
            | Action::Dragging { animations, .. } => {
                animations.wrap_extent = max_width;
                animations.cross_spacing = vertical_spacing;
            }
        }

        let mut children: Vec<layout::Node> = Vec::new();
        let mut intrinsic_size = Size::ZERO;
        let mut row_start = 0;
        let mut row_height = 0.0;
        let mut x = 0.0;
        let mut y = 0.0;

        let align_factor = match self.row.align {
            Alignment::Start => 0.0,
            Alignment::Center => 2.0,
            Alignment::End => 1.0,
        };

        let align_y = |row_start: std::ops::Range<usize>,
                       row_height: f32,
                       children: &mut Vec<layout::Node>| {
            if align_factor != 0.0 {
                for node in &mut children[row_start] {
                    let height = node.size().height;

                    node.translate_mut(Vector::new(
                        0.0,
                        (row_height - height) / align_factor,
                    ));
                }
            }
        };

        for (i, child) in self.row.children.iter_mut().enumerate() {
            let node = child.as_widget_mut().layout(
                &mut tree.children[i],
                renderer,
                &child_limits,
            );

            let child_size = node.size();

            if x != 0.0 && x + child_size.width > max_width {
                intrinsic_size.width = intrinsic_size.width.max(x - spacing);

                align_y(row_start..i, row_height, &mut children);

                y += row_height + vertical_spacing;
                x = 0.0;
                row_start = i;
                row_height = 0.0;
            }

            row_height = row_height.max(child_size.height);

            children.push(node.move_to((
                x + self.row.padding.left,
                y + self.row.padding.top,
            )));

            x += child_size.width + spacing;
        }

        if x != 0.0 {
            intrinsic_size.width = intrinsic_size.width.max(x - spacing);
        }

        intrinsic_size.height = y + row_height;
        align_y(row_start..children.len(), row_height, &mut children);

        let align_factor = match self.align_x {
            alignment::Horizontal::Left => 0.0,
            alignment::Horizontal::Center => 2.0,
            alignment::Horizontal::Right => 1.0,
        };

        if align_factor != 0.0 {
            let total_width = intrinsic_size.width;

            let mut row_start = 0;

            for i in 0..children.len() {
                let bounds = children[i].bounds();
                let row_width = bounds.x + bounds.width;

                let next_x = children
                    .get(i + 1)
                    .map(|node| node.bounds().x)
                    .unwrap_or_default();

                if next_x == 0.0 {
                    let translation = Vector::new(
                        (total_width - row_width) / align_factor,
                        0.0,
                    );

                    for node in &mut children[row_start..=i] {
                        node.translate_mut(translation);
                    }

                    row_start = i + 1;
                }
            }
        }

        let size =
            limits.resolve(self.row.width, self.row.height, intrinsic_size);

        if self.row.id.is_some() {
            let state = tree.state.downcast_mut::<WidgetState>();
            state.positions.clear();
            for (i, child) in children.iter().enumerate() {
                state.positions.set(i, child.bounds());
            }
        }

        layout::Node::with_children(size.expand(self.row.padding), children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.row.operate(tree, layout, renderer, operation);
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
        self.row.update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.row
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.row
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.row
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<Wrapping<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(row: Wrapping<'a, Message, Theme, Renderer>) -> Self {
        Self::new(row)
    }
}

/// The theme catalog of a [`Row`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// The appearance of a [`Row`] during drag operations.
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

/// A styling function for a [`Row`].
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

/// The default style for a [`Row`].
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
