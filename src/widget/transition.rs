//! A widget that animates content swaps with a slide transition.
//!
//! A [`Transition`] holds a single child [`Element`] derived from a value of
//! type `T`. When the value changes, the new content slides into view from
//! one edge while the previous content slides out the opposite edge — like
//! Compose's `AnimatedContent` or Android's `ViewSwitcher`.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! use iced::widget::text;
//! use sweeten::widget::transition::{self, Direction};
//!
//! struct State { phrase: String }
//! enum Message {}
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     transition::transition(state.phrase.clone(), |s: &String| {
//!         text(s.clone()).size(24).into()
//!     })
//!     .direction(Direction::Up)
//!     .into()
//! }
//! ```
//!
//! The closure receives the current value (or, mid-animation, the previous
//! value) and produces an [`Element`]. Because the produced [`Element`] must
//! have lifetime `'a`, the closure body cannot borrow from its `&T` argument
//! directly — clone the data inside the closure or use captures.

use std::time::Duration;

use crate::core::animation::Easing;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::widget::Operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Alignment, Animation, Element, Event, Length, Point, Rectangle, Shell,
    Size, Vector, Widget,
};

/// The direction of the slide motion when content swaps.
///
/// The new content enters from the side opposite the motion direction and
/// the previous content exits along the same direction. For example,
/// [`Direction::Up`] makes the new content appear from the bottom edge and
/// slide upward into the canonical position, pushing the previous content
/// off the top edge.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Motion is upward. New content enters from the bottom edge.
    #[default]
    Up,
    /// Motion is downward. New content enters from the top edge.
    Down,
    /// Motion is leftward. New content enters from the right edge.
    Left,
    /// Motion is rightward. New content enters from the left edge.
    Right,
}

impl Direction {
    /// Returns the unit vector of motion as `(dx, dy)` in the iced
    /// coordinate system (y grows downward).
    fn unit(self) -> (f32, f32) {
        match self {
            Self::Up => (0.0, -1.0),
            Self::Down => (0.0, 1.0),
            Self::Left => (-1.0, 0.0),
            Self::Right => (1.0, 0.0),
        }
    }

    /// Returns `(current_offset, previous_offset)` for the given widget
    /// `size` and animation `progress` in `[0, 1]`.
    fn offsets(self, size: Size, progress: f32) -> (Vector, Vector) {
        let (ux, uy) = self.unit();
        let dx = size.width;
        let dy = size.height;
        let inv = 1.0 - progress;
        let current = Vector::new(-ux * dx * inv, -uy * dy * inv);
        let previous = Vector::new(ux * dx * progress, uy * dy * progress);
        (current, previous)
    }
}

/// A boxed view function: takes the current value of `T` and produces an
/// [`Element`] for it. Used by [`Transition`] to materialize child elements
/// from both the current and (mid-transition) previous values.
type ViewFn<'a, T, Message, Theme, Renderer> =
    Box<dyn Fn(&T) -> Element<'a, Message, Theme, Renderer> + 'a>;

/// A widget that animates content swaps with a slide transition.
///
/// See the [module-level documentation][self] for an example.
pub struct Transition<
    'a,
    T,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    T: Clone + PartialEq + 'static,
    Renderer: crate::core::Renderer,
{
    value: T,
    view: ViewFn<'a, T, Message, Theme, Renderer>,
    direction: Direction,
    duration: Duration,
    easing: Easing,
    width: Length,
    height: Length,
    /// Lazy slot used by [`Widget::overlay`] to return a borrow with
    /// lifetime `'a`. Populated on demand and dropped when the widget
    /// is dropped at the end of the frame.
    overlay_element: Option<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, T, Message, Theme, Renderer>
    Transition<'a, T, Message, Theme, Renderer>
where
    T: Clone + PartialEq + 'static,
    Renderer: crate::core::Renderer,
{
    /// Creates a new [`Transition`] showing the given `value`, with `view` as
    /// the recipe for materializing an [`Element`] from any value of type
    /// `T`.
    ///
    /// Whenever `value` changes between frames (as detected by [`PartialEq`]),
    /// the widget will animate a slide transition between the previous and
    /// new content.
    ///
    /// The closure must produce an [`Element`] of lifetime `'a` — it cannot
    /// borrow from its `&T` argument directly. Clone the data inside the
    /// closure or use captures of lifetime `'a`.
    pub fn new(
        value: T,
        view: impl Fn(&T) -> Element<'a, Message, Theme, Renderer> + 'a,
    ) -> Self {
        Self {
            value,
            view: Box::new(view),
            direction: Direction::default(),
            duration: Duration::from_millis(200),
            easing: Easing::EaseOut,
            width: Length::Shrink,
            height: Length::Shrink,
            overlay_element: None,
        }
    }

    /// Sets the [`Direction`] of the slide motion.
    #[must_use]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the [`Duration`] of the slide animation.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Sets the [`Easing`] function of the slide animation.
    #[must_use]
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Sets the width of the [`Transition`].
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Transition`].
    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

/// Internal state for the [`Transition`] widget.
struct State<T> {
    current_value: T,
    previous_value: Option<T>,
    current_tree: Tree,
    previous_tree: Tree,
    /// Frozen [`layout::Node`] for the current child. Computed exactly
    /// once when the child enters (i.e., on the first [`Widget::layout`]
    /// after a swap or after initialization) and never recomputed —
    /// recomputing would re-wrap text and shift positions mid-slide.
    current_layout: Option<layout::Node>,
    /// Frozen [`layout::Node`] for the outgoing child during a slide.
    /// Promoted from `current_layout` in [`Widget::diff`] at the moment
    /// of the swap. Cleared when the animation completes.
    previous_layout: Option<layout::Node>,
    progress: Animation<f32>,
    /// Set in [`Widget::diff`] when a swap is detected; consumed in
    /// [`Widget::update`] on the next [`window::Event::RedrawRequested`]
    /// (which is the first place we have an [`Instant`] to arm the
    /// animation with).
    pending_start: bool,
    now: Option<Instant>,
}

impl<T> State<T> {
    fn drop_previous(&mut self) {
        self.previous_value = None;
        self.previous_tree = Tree::empty();
        self.previous_layout = None;
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Transition<'a, T, Message, Theme, Renderer>
where
    T: Clone + PartialEq + 'static,
    Renderer: crate::core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<T>>()
    }

    fn state(&self) -> tree::State {
        let element = (self.view)(&self.value);
        tree::State::new(State::<T> {
            current_value: self.value.clone(),
            previous_value: None,
            current_tree: Tree::new(element.as_widget()),
            previous_tree: Tree::empty(),
            current_layout: None,
            previous_layout: None,
            progress: Animation::new(0.0_f32)
                .duration(self.duration)
                .easing(self.easing),
            pending_start: false,
            now: None,
        })
    }

    fn children(&self) -> Vec<Tree> {
        // We manage child trees manually inside `State<T>`, not via the
        // standard `tree.children` machinery. This avoids structural
        // mismatches when the previous child appears or disappears.
        Vec::new()
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State<T>>();

        if state.current_value != self.value {
            // Snap-and-restart: any in-flight previous is dropped, the
            // currently-showing value becomes the new previous, and the new
            // value becomes current. The outgoing child's frozen layout is
            // promoted directly from `current_layout` — we never recompute
            // it for the rest of the slide. Animation is reset and will be
            // armed on the next `RedrawRequested` (which carries an
            // `Instant`).
            let old_value =
                std::mem::replace(&mut state.current_value, self.value.clone());
            let old_tree =
                std::mem::replace(&mut state.current_tree, Tree::empty());

            state.previous_value = Some(old_value);
            state.previous_tree = old_tree;
            state.previous_layout = state.current_layout.take();
            // current_layout is now None — the new current will be laid
            // out exactly once on the next call to `Widget::layout`.

            let element = (self.view)(&state.current_value);
            state.current_tree = Tree::new(element.as_widget());
            state.current_tree.diff(element.as_widget());

            state.progress = Animation::new(0.0_f32)
                .duration(self.duration)
                .easing(self.easing);
            state.pending_start = true;
        } else {
            // No swap; reconcile current child against a fresh element so
            // any unrelated changes (e.g., a captured field updating) still
            // propagate to the child's tree state.
            let element = (self.view)(&state.current_value);
            state.current_tree.diff(element.as_widget());
        }

        // Reconcile the outgoing child too while it's still on screen.
        if let Some(prev) = state.previous_value.as_ref() {
            let prev_element = (self.view)(prev);
            state.previous_tree.diff(prev_element.as_widget());
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
        let state = tree.state.downcast_mut::<State<T>>();
        let limits = limits.width(self.width).height(self.height);

        // Lay out the current child exactly once on entry. After this it
        // stays frozen until the next swap (or forever, if no swap ever
        // happens). Subsequent layout passes reuse the frozen node.
        if state.current_layout.is_none() {
            let mut current_element = (self.view)(&state.current_value);
            let current_node = current_element.as_widget_mut().layout(
                &mut state.current_tree,
                renderer,
                &limits,
            );
            state.current_layout = Some(current_node);
        }

        // Fallback: if a swap happened before any layout pass had ever
        // run on the prior current (so `current_layout` was still `None`
        // when `diff` tried to promote it), lay the previous out now.
        // After the first layout call this branch never fires again.
        if state.previous_value.is_some()
            && state.previous_layout.is_none()
            && let Some(prev_value) = state.previous_value.clone()
        {
            let mut prev_element = (self.view)(&prev_value);
            let prev_node = prev_element.as_widget_mut().layout(
                &mut state.previous_tree,
                renderer,
                &limits,
            );
            state.previous_layout = Some(prev_node);
        }

        let current_node = state
            .current_layout
            .as_ref()
            .expect("current_layout was just set above");
        let current_size = current_node.size();

        // Widget size has to fit BOTH children during the slide, otherwise
        // the `with_layer` clip in `draw` trims the outgoing child to the
        // smaller bounds. After the animation completes and we drop the
        // previous, the widget snaps back to the current child's size.
        let widget_size = match state.previous_layout.as_ref() {
            Some(prev_node) => {
                let prev_size = prev_node.size();
                Size::new(
                    current_size.width.max(prev_size.width),
                    current_size.height.max(prev_size.height),
                )
            }
            None => current_size,
        };

        // Position the current child within the widget bounds.
        //
        // - Steady state (no previous, widget == current's size): top-left
        //   of widget bounds. The parent's container places the widget,
        //   our top-left lands wherever the parent puts us.
        //
        // - Mid-animation (widget == max(prev, cur)): center the current
        //   within the larger widget bounds. Combined with a centering
        //   parent, this places the current at the same absolute position
        //   it'll occupy after the animation completes and the widget
        //   snaps back to its own size — so there's no visible "juke" to
        //   the final cross-axis (or main-axis) position when previous is
        //   dropped. Without this, the current sits at top-left of the
        //   max bounds during the slide and jumps to center on the last
        //   frame.
        let mut current_child = current_node.clone();
        if state.previous_layout.is_some() {
            current_child.move_to_mut(Point::ORIGIN);
            current_child.align_mut(
                Alignment::Center,
                Alignment::Center,
                widget_size,
            );
        }

        layout::Node::with_children(widget_size, vec![current_child])
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
        let state = tree.state.downcast_mut::<State<T>>();

        // Animation bookkeeping. Done first so the rest of the call sees
        // up-to-date `now` / animation state.
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            state.now = Some(*now);

            if state.pending_start {
                state.progress.go_mut(1.0, *now);
                state.pending_start = false;
                shell.request_redraw();
            }

            if state.progress.is_animating(*now) {
                shell.request_redraw();
            } else if state.previous_value.is_some() && !state.pending_start {
                // Animation just finished — release the outgoing child.
                // Our reported size shrinks from `max(prev, cur)` back to
                // `current`'s size; the layout that ran in `build()` for
                // this frame used the larger size and is now stale.
                // Invalidating tells iced to re-run layout in the same
                // update cycle (via `revalidate_layout`) so the draw call
                // sees the smaller bounds.
                state.drop_previous();
                shell.invalidate_layout();
            }
        }

        // Forward the event to the current child only. The outgoing child
        // is decorative; routing events into a sliding-out widget is a
        // footgun. Note that we use the un-translated layout — events fire
        // as if the current child were already at its final position. For a
        // 200ms animation this is imperceptible.
        let mut current_element = (self.view)(&state.current_value);
        if let Some(current_layout) = layout.children().next() {
            current_element.as_widget_mut().update(
                &mut state.current_tree,
                event,
                current_layout,
                cursor,
                renderer,
                shell,
                viewport,
            );
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
        let state = tree.state.downcast_ref::<State<T>>();
        let current_element = (self.view)(&state.current_value);
        let Some(current_layout) = layout.children().next() else {
            return mouse::Interaction::None;
        };
        current_element.as_widget().mouse_interaction(
            &state.current_tree,
            current_layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State<T>>();
        let mut current_element = (self.view)(&state.current_value);
        let Some(current_layout) = layout.children().next() else {
            return;
        };
        current_element.as_widget_mut().operate(
            &mut state.current_tree,
            current_layout,
            renderer,
            operation,
        );
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
        let state = tree.state.downcast_ref::<State<T>>();
        let bounds = layout.bounds();
        let Some(current_layout) = layout.children().next() else {
            return;
        };

        let progress = match state.now {
            Some(now) => state.progress.interpolate_with(|v| v, now),
            // No frame has happened yet — render the current child at its
            // final position with no offset.
            None => 1.0,
        };

        let has_previous =
            state.previous_value.is_some() && state.previous_layout.is_some();

        let (current_offset, previous_offset) = if has_previous {
            self.direction.offsets(bounds.size(), progress)
        } else {
            (Vector::ZERO, Vector::ZERO)
        };

        renderer.with_layer(bounds, |renderer| {
            // Draw the outgoing child first so the incoming child paints on
            // top of it at the seam.
            if has_previous
                && let Some(prev_value) = state.previous_value.as_ref()
                && let Some(prev_node) = state.previous_layout.as_ref()
            {
                let prev_element = (self.view)(prev_value);
                // Position the frozen previous layout centered within the
                // widget's max bounds — same idea as the current child in
                // `layout()`. This keeps the previous at the same absolute
                // position it had pre-swap (assuming a centering parent),
                // so there's no juke at the moment of the swap either.
                let prev_size = prev_node.size();
                let prev_origin = Vector::new(
                    bounds.x + (bounds.width - prev_size.width) / 2.0,
                    bounds.y + (bounds.height - prev_size.height) / 2.0,
                );
                let prev_layout = Layout::with_offset(prev_origin, prev_node);
                renderer.with_translation(previous_offset, |renderer| {
                    prev_element.as_widget().draw(
                        &state.previous_tree,
                        renderer,
                        theme,
                        defaults,
                        prev_layout,
                        cursor,
                        viewport,
                    );
                });
            }

            let current_element = (self.view)(&state.current_value);
            renderer.with_translation(current_offset, |renderer| {
                current_element.as_widget().draw(
                    &state.current_tree,
                    renderer,
                    theme,
                    defaults,
                    current_layout,
                    cursor,
                    viewport,
                );
            });
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State<T>>();
        // Materialize and stash the current element on `self` so the
        // returned overlay reference outlives this call (it borrows from
        // `self.overlay_element`, which lives `'b`).
        self.overlay_element = Some((self.view)(&state.current_value));
        let element = self.overlay_element.as_mut()?;
        let current_layout = layout.children().next()?;
        element.as_widget_mut().overlay(
            &mut state.current_tree,
            current_layout,
            renderer,
            viewport,
            translation,
        )
    }
}

/// Creates a new [`Transition`] showing the given `value`, with `view` as the
/// recipe for materializing an [`Element`] from any value of type `T`.
///
/// This is the helper-style alias of [`Transition::new`].
pub fn transition<'a, T, Message, Theme, Renderer>(
    value: T,
    view: impl Fn(&T) -> Element<'a, Message, Theme, Renderer> + 'a,
) -> Transition<'a, T, Message, Theme, Renderer>
where
    T: Clone + PartialEq + 'static,
    Renderer: crate::core::Renderer,
{
    Transition::new(value, view)
}

impl<'a, T, Message, Theme, Renderer>
    From<Transition<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Clone + PartialEq + 'static,
    Message: 'a,
    Theme: 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(
        widget: Transition<'a, T, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(widget)
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Up => write!(f, "Up"),
            Direction::Down => write!(f, "Down"),
            Direction::Left => write!(f, "Left"),
            Direction::Right => write!(f, "Right"),
        }
    }
}
