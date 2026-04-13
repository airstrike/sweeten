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

use crate::core::alignment;
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
    Alignment, Animation, Element, Event, Length, Padding, Pixels, Rectangle,
    Shell, Size, Vector, Widget,
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

    /// Returns `(current_offset, previous_offset)` for the given
    /// content area `size` and animation `progress` in `[0, 1]`.
    ///
    /// Displacement equals the content area's extent along the motion
    /// axis so both children fully enter/exit the content area at their
    /// respective end times.
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
    max_width: f32,
    max_height: f32,
    padding: Padding,
    horizontal_alignment: alignment::Horizontal,
    vertical_alignment: alignment::Vertical,
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
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            padding: Padding::ZERO,
            // Default to centered alignment. This differs from iced's
            // [`Container`] (which defaults to top-left) because the
            // common use case for a [`Transition`] is a banner-style
            // centered slot, and because a centered alignment cancels
            // the juke that would otherwise appear when the widget's
            // reported size shrinks after the animation completes in a
            // centered parent.
            horizontal_alignment: alignment::Horizontal::Center,
            vertical_alignment: alignment::Vertical::Center,
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

    /// Sets the maximum width of the [`Transition`].
    #[must_use]
    pub fn max_width(mut self, max_width: impl Into<Pixels>) -> Self {
        self.max_width = max_width.into().0;
        self
    }

    /// Sets the maximum height of the [`Transition`].
    #[must_use]
    pub fn max_height(mut self, max_height: impl Into<Pixels>) -> Self {
        self.max_height = max_height.into().0;
        self
    }

    /// Sets the [`Padding`] of the [`Transition`].
    #[must_use]
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the content alignment for the horizontal axis.
    ///
    /// This determines where both the current and the outgoing children
    /// sit along the cross/main axis within the [`Transition`]'s content
    /// area. For a [`Transition`] inside a centered parent, leave this
    /// at the default [`alignment::Horizontal::Center`] to avoid a juke
    /// when the widget snaps back to the current child's size at the end
    /// of the animation.
    #[must_use]
    pub fn align_x(
        mut self,
        alignment: impl Into<alignment::Horizontal>,
    ) -> Self {
        self.horizontal_alignment = alignment.into();
        self
    }

    /// Sets the content alignment for the vertical axis.
    ///
    /// See [`align_x`](Self::align_x) for the rationale around choosing
    /// an alignment that matches the parent's.
    #[must_use]
    pub fn align_y(
        mut self,
        alignment: impl Into<alignment::Vertical>,
    ) -> Self {
        self.vertical_alignment = alignment.into();
        self
    }

    /// Sets the width of the [`Transition`] and centers its contents
    /// horizontally.
    #[must_use]
    pub fn center_x(self, width: impl Into<Length>) -> Self {
        self.width(width).align_x(alignment::Horizontal::Center)
    }

    /// Sets the height of the [`Transition`] and centers its contents
    /// vertically.
    #[must_use]
    pub fn center_y(self, height: impl Into<Length>) -> Self {
        self.height(height).align_y(alignment::Vertical::Center)
    }

    /// Sets the width and height of the [`Transition`] and centers its
    /// contents on both axes.
    #[must_use]
    pub fn center(self, length: impl Into<Length>) -> Self {
        let length = length.into();
        self.center_x(length).center_y(length)
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
        let h_align = Alignment::from(self.horizontal_alignment);
        let v_align = Alignment::from(self.vertical_alignment);

        // Defer to [`layout::positioned`] for the standard
        // width/height/max/padding/alignment plumbing (mirroring
        // [`iced_widget::container`]'s Widget::layout). The only
        // wrinkle: `positioned` uses the returned content node's size
        // to drive padding.fit + resolve, and we want those to see the
        // *max* of the current and the in-flight previous child — so
        // the closure returns a wrapper node sized to `content_size`
        // with the current child aligned inside it. The outer
        // positioning pass then aligns that wrapper inside the resolved
        // area; two levels of alignment with the same (h, v) collapses
        // to aligning the current directly within the resolved area.
        layout::positioned(
            &limits.max_width(self.max_width).max_height(self.max_height),
            self.width,
            self.height,
            self.padding,
            |inner_limits| {
                let inner_limits = inner_limits.loose();

                // Lay out the current child exactly once on entry.
                // Subsequent layout passes reuse the frozen node.
                if state.current_layout.is_none() {
                    let mut current_element = (self.view)(&state.current_value);
                    let node = current_element.as_widget_mut().layout(
                        &mut state.current_tree,
                        renderer,
                        &inner_limits,
                    );
                    state.current_layout = Some(node);
                }

                // Fallback: if a swap happened before any layout pass
                // had ever run on the prior current (so `current_layout`
                // was still `None` when `diff` tried to promote it),
                // lay the previous out now. After the first layout call
                // this branch never fires again.
                if state.previous_value.is_some()
                    && state.previous_layout.is_none()
                    && let Some(prev_value) = state.previous_value.clone()
                {
                    let mut prev_element = (self.view)(&prev_value);
                    let node = prev_element.as_widget_mut().layout(
                        &mut state.previous_tree,
                        renderer,
                        &inner_limits,
                    );
                    state.previous_layout = Some(node);
                }

                let current_node = state
                    .current_layout
                    .as_ref()
                    .expect("current_layout was just set above");
                let current_size = current_node.size();

                // Content size = max of both children along each axis.
                // In steady state this collapses to current_size.
                let content_size = match state.previous_layout.as_ref() {
                    Some(prev) => {
                        let prev_size = prev.size();
                        Size::new(
                            current_size.width.max(prev_size.width),
                            current_size.height.max(prev_size.height),
                        )
                    }
                    None => current_size,
                };

                // Wrap the current child in a `content_size`-sized
                // parent, with the current aligned inside.
                let mut current_inside = current_node.clone();
                current_inside.align_mut(h_align, v_align, content_size);
                layout::Node::with_children(content_size, vec![current_inside])
            },
            |content, size| content.align(h_align, v_align, size),
        )
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
        //
        // Our layout tree is Outer → Wrapper → Current, so we have to
        // step through the wrapper to reach the current child's layout.
        let mut current_element = (self.view)(&state.current_value);
        if let Some(current_layout) = layout
            .children()
            .next()
            .and_then(|wrapper| wrapper.children().next())
        {
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
        let Some(current_layout) = layout
            .children()
            .next()
            .and_then(|wrapper| wrapper.children().next())
        else {
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
        let Some(current_layout) = layout
            .children()
            .next()
            .and_then(|wrapper| wrapper.children().next())
        else {
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
        // Content area = outer bounds minus padding. Unlike the
        // wrapper's bounds (which is `max(prev, cur)` and can be much
        // smaller than the widget itself under Fill sizing), this
        // captures the *full region* within which the slide runs. It
        // drives both the slide displacement — so that a 1-line text in
        // a 1000-tall widget travels 1000pt rather than 22pt — and the
        // clip rect, so sliding children fill the whole widget.
        let outer_bounds = layout.bounds();
        let content_area = Rectangle {
            x: outer_bounds.x + self.padding.left,
            y: outer_bounds.y + self.padding.top,
            width: (outer_bounds.width - self.padding.x()).max(0.0),
            height: (outer_bounds.height - self.padding.y()).max(0.0),
        };
        // Outer → Wrapper → Current navigation.
        let Some(wrapper_layout) = layout.children().next() else {
            return;
        };
        let Some(current_layout) = wrapper_layout.children().next() else {
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
            self.direction.offsets(content_area.size(), progress)
        } else {
            (Vector::ZERO, Vector::ZERO)
        };

        // Clip to the content area so sliding children don't bleed into
        // the padding region or adjacent siblings.
        renderer.with_layer(content_area, |renderer| {
            // Draw the outgoing child first so the incoming child paints
            // on top of it at the seam.
            if has_previous
                && let Some(prev_value) = state.previous_value.as_ref()
                && let Some(prev_node) = state.previous_layout.as_ref()
            {
                let prev_element = (self.view)(prev_value);
                // Position the frozen previous aligned within the full
                // content area (not within the wrapper). For same
                // alignment, this matches the current child's canonical
                // absolute position exactly — so the two children share
                // the same starting/ending reference, and they both
                // travel the full content-area extent.
                let mut prev_positioned = prev_node.clone();
                prev_positioned.align_mut(
                    Alignment::from(self.horizontal_alignment),
                    Alignment::from(self.vertical_alignment),
                    content_area.size(),
                );
                let prev_layout = Layout::with_offset(
                    Vector::new(content_area.x, content_area.y),
                    &prev_positioned,
                );
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
        let current_layout = layout
            .children()
            .next()
            .and_then(|wrapper| wrapper.children().next())?;
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
