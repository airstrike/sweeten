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

/// The transition style applied when content swaps.
///
/// New variants slot in here; each gets its own private `*_transforms`
/// function below that produces the visual transforms for both children
/// at a given animation progress. Slide is the only mode today; future
/// modes (`Crossfade`, `Fade`, `Wipe`, `Hero`, …) will extend [`Content`]
/// with the additional knobs they need (alpha for the fades, a per-child
/// clip rect for wipe, etc.) and add a match arm in [`transforms`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Slide the new content into the canonical position from the edge
    /// opposite the [`Direction`], pushing the previous content off the
    /// same-side edge.
    Slide(Direction),
}

impl Default for Mode {
    fn default() -> Self {
        Self::Slide(Direction::default())
    }
}

/// Per-pass visual transform for one of the two children rendered during
/// a transition. Mode-agnostic shape: modes that need additional knobs
/// (alpha for fade/crossfade, per-child clip rect for wipe) extend this
/// struct, and [`Widget::draw`] picks the new fields up.
#[derive(Debug, Clone, Copy, Default)]
struct Content {
    translation: Vector,
}

/// Pair of [`Content`] transforms — one for the entering child, one for
/// the exiting child. Returned by [`transforms`].
#[derive(Debug, Clone, Copy, Default)]
struct Transforms {
    current: Content,
    previous: Content,
}

/// Computes the per-child transforms for `mode` at the given animation
/// `progress` in `[0, 1]`, given the content area `size`. Pure function;
/// the only place that knows how each mode renders.
fn transforms(mode: Mode, size: Size, progress: f32) -> Transforms {
    match mode {
        Mode::Slide(direction) => slide_transforms(direction, size, progress),
    }
}

/// Slide-specific [`Transforms`].
///
/// Both translations have magnitude equal to the content area's extent
/// along the motion axis, so the current slides all the way in from the
/// edge opposite the motion direction, and the previous slides all the
/// way out along the motion direction.
///
/// The translations are *visual*: layout puts the current child at its
/// canonical (non-translated) position, so events, focus, and
/// hit-testing fire there from t=0. `draw` applies these via
/// `Renderer::with_translation`, and `update` / `mouse_interaction`
/// translate the incoming cursor by `-current.translation` so the
/// child's hover/click tests match what the user sees.
fn slide_transforms(
    direction: Direction,
    size: Size,
    progress: f32,
) -> Transforms {
    let (ux, uy) = match direction {
        Direction::Up => (0.0, -1.0),
        Direction::Down => (0.0, 1.0),
        Direction::Left => (-1.0, 0.0),
        Direction::Right => (1.0, 0.0),
    };
    let inv = 1.0 - progress;
    let current = Vector::new(-ux * size.width * inv, -uy * size.height * inv);
    let previous =
        Vector::new(ux * size.width * progress, uy * size.height * progress);
    Transforms {
        current: Content {
            translation: current,
        },
        previous: Content {
            translation: previous,
        },
    }
}

/// A boxed view function: takes the current value of `T` and produces an
/// [`Element`] for it. Used by [`Transition`] to materialize child elements
/// from both the current and (mid-transition) previous values.
type ViewFn<'a, T, Message, Theme, Renderer> =
    Box<dyn Fn(&T) -> Element<'a, Message, Theme, Renderer> + 'a>;

/// Recomputes the current child's visual translation from the widget's
/// absolute bounds, padding, and animation state. Shared by `update` and
/// `mouse_interaction` for cursor-translation; `draw` reads both halves
/// of [`transforms`] directly.
fn current_offset<T>(
    mode: Mode,
    padding: Padding,
    layout: &Layout<'_>,
    state: &State<T>,
) -> Vector {
    if state.previous_value.is_none() || state.previous_layout.is_none() {
        return Vector::ZERO;
    }
    let Some(now) = state.now else {
        return Vector::ZERO;
    };
    let progress = state.progress.interpolate_with(|v| v, now);
    let outer = layout.bounds();
    let content_size = Size::new(
        (outer.width - padding.x()).max(0.0),
        (outer.height - padding.y()).max(0.0),
    );
    transforms(mode, content_size, progress).current.translation
}

/// Subtracts a slide offset from a cursor's screen position so a child
/// laid out at its canonical position can hit-test against the visual
/// (translated) rendering.
fn translate_cursor(cursor: mouse::Cursor, offset: Vector) -> mouse::Cursor {
    match cursor {
        mouse::Cursor::Available(point) => {
            mouse::Cursor::Available(point - offset)
        }
        other => other,
    }
}

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
    mode: Mode,
    duration: Duration,
    easing: Easing,
    width: Length,
    height: Length,
    max_width: f32,
    max_height: f32,
    padding: Padding,
    horizontal_alignment: alignment::Horizontal,
    vertical_alignment: alignment::Vertical,
    /// The live current child, materialized exactly once per frame in
    /// the first [`Widget`] callback that runs ([`Widget::layout`]) and
    /// reused in [`Widget::update`], [`Widget::draw`],
    /// [`Widget::mouse_interaction`], [`Widget::operate`], and
    /// [`Widget::overlay`]. Sharing the instance is essential for
    /// child widgets that persist state on the widget struct itself
    /// rather than in [`tree::State`] — e.g. iced's
    /// [`button`](iced_widget::button)'s `status` field, or
    /// [`toggler`](crate::widget::toggler)'s `last_status` — since
    /// those get set in the child's `update` and read in its `draw`,
    /// and would be lost if we re-materialized a fresh element in
    /// between.
    ///
    /// The slot is `None` until the first frame callback runs; freshly
    /// reset every frame when iced rebuilds the widget tree via
    /// `view()`.
    current_element: Option<Element<'a, Message, Theme, Renderer>>,
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
            mode: Mode::default(),
            duration: Duration::from_millis(200),
            easing: Easing::EaseOut,
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
            padding: Padding::ZERO,
            // Default to center rather than top-left (iced
            // `Container`'s default). When the animation completes
            // and the widget shrinks back to the current child's
            // size, a centering parent will re-center it; any
            // alignment mismatch between parent and `Transition`
            // shows up at that moment as a visual snap.
            horizontal_alignment: alignment::Horizontal::Center,
            vertical_alignment: alignment::Vertical::Center,
            current_element: None,
        }
    }

    /// Sets the [`Mode`] of the transition.
    #[must_use]
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the [`Direction`] of the slide motion. Sugar for
    /// `.mode(Mode::Slide(direction))`, the most common case.
    #[must_use]
    pub fn direction(self, direction: Direction) -> Self {
        self.mode(Mode::Slide(direction))
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
    /// Refreshed each [`Widget::layout`] call; moved into
    /// `previous_layout` on swap.
    current_layout: Option<layout::Node>,
    /// Frozen at swap time and held for the animation's duration.
    /// Reflowing it would jitter the geometry that [`Widget::draw`]
    /// is translating.
    previous_layout: Option<layout::Node>,
    progress: Animation<f32>,
    /// Arming [`progress`] requires an [`Instant`], which only
    /// [`window::Event::RedrawRequested`] carries. Set in
    /// [`Widget::diff`], consumed in [`Widget::update`].
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
        // Child trees live on `State<T>` directly. The slot-based
        // machinery doesn't handle a changing child count across
        // swaps without tearing down tree state each time.
        Vec::new()
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<State<T>>();

        if state.current_value != self.value {
            // Snap-and-restart: any in-flight previous is dropped,
            // the current becomes the new previous (its layout
            // promoted directly from `current_layout`), and the new
            // value becomes current.
            let old_value =
                std::mem::replace(&mut state.current_value, self.value.clone());
            let old_tree =
                std::mem::replace(&mut state.current_tree, Tree::empty());

            state.previous_value = Some(old_value);
            state.previous_tree = old_tree;
            state.previous_layout = state.current_layout.take();

            let element = (self.view)(&state.current_value);
            state.current_tree = Tree::new(element.as_widget());
            state.current_tree.diff(element.as_widget());

            state.progress = Animation::new(0.0_f32)
                .duration(self.duration)
                .easing(self.easing);
            state.pending_start = true;
        } else {
            let element = (self.view)(&state.current_value);
            state.current_tree.diff(element.as_widget());
        }

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

        // Stash the materialized current child on `self` so every
        // subsequent callback reaches the same instance. Required
        // for child widgets that persist state on `self` rather
        // than in `tree::State` — e.g. iced's `button.status` and
        // `toggler.last_status`, set in the child's `update` and
        // read in its `draw`.
        if self.current_element.is_none() {
            self.current_element = Some((self.view)(&state.current_value));
        }

        let h_align = Alignment::from(self.horizontal_alignment);
        let v_align = Alignment::from(self.vertical_alignment);

        // Split-borrow `self` into two disjoint fields the closure
        // below needs: `current_element` (mut) and `view` (shared,
        // for the previous-layout fallback). A single `&mut self`
        // capture would conflict.
        let view = &self.view;
        let current_element = self
            .current_element
            .as_mut()
            .expect("current_element just set above");

        // Defer to [`layout::positioned`] (same pattern as
        // [`iced_widget::container`]'s `Widget::layout`) for the
        // width/height/max/padding/alignment plumbing. The wrinkle:
        // `positioned` drives `padding.fit`/`resolve` from the
        // content node's size, and we want those to see the *max*
        // of current and any in-flight previous. The closure
        // returns a wrapper node sized to `content_size` with the
        // current aligned inside it; `positioned` then aligns that
        // wrapper inside the resolved area. Two levels of alignment
        // with the same `(h, v)` collapse to aligning the current
        // directly within the resolved area.
        layout::positioned(
            &limits.max_width(self.max_width).max_height(self.max_height),
            self.width,
            self.height,
            self.padding,
            |inner_limits| {
                let inner_limits = inner_limits.loose();

                // Re-run layout on the current child every call.
                // This is what triggers iced's child widgets to
                // refresh their own tree-state caches — notably
                // `text`'s `paragraph.update()`, which reshapes the
                // cached paragraph when the content string changes.
                // Only the *previous* child's layout is frozen; it's
                // a snapshot we're animating out, not reactive.
                let node = current_element.as_widget_mut().layout(
                    &mut state.current_tree,
                    renderer,
                    &inner_limits,
                );
                state.current_layout = Some(node);

                // Fallback: swap happened before any layout call had
                // run on the prior current, so `diff` had nothing to
                // promote into `previous_layout`.
                if state.previous_value.is_some()
                    && state.previous_layout.is_none()
                    && let Some(prev_value) = state.previous_value.clone()
                {
                    let mut prev_element = view(&prev_value);
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

                // Per-axis max of both children, so the content area
                // has room for both during a slide. Collapses to the
                // current's size in steady state.
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
                // Animation just finished; release the outgoing
                // child. The widget's reported size will shrink from
                // `max(prev, cur)` back to the current's size, but
                // the layout that ran earlier in this update cycle
                // used the larger size. `invalidate_layout` asks
                // iced to re-run layout before `draw` (via
                // `revalidate_layout`) so `draw` sees the smaller
                // bounds.
                state.drop_previous();
                shell.invalidate_layout();
            }
        }

        // `current_layout` is at the child's un-translated canonical
        // position, where events fire from. Subtracting
        // `current_offset` from the cursor makes hit-testing line up
        // with the visual position `draw` paints at.
        let current_offset =
            current_offset(self.mode, self.padding, &layout, state);

        // Defensive: `layout` runs before `update` in the normal
        // path, but handle the inverted order so events never drop.
        if self.current_element.is_none() {
            self.current_element = Some((self.view)(&state.current_value));
        }
        let current_element = self
            .current_element
            .as_mut()
            .expect("just materialized above");
        if let Some(current_layout) = layout
            .children()
            .next()
            .and_then(|wrapper| wrapper.children().next())
        {
            let adjusted_cursor = translate_cursor(cursor, current_offset);
            current_element.as_widget_mut().update(
                &mut state.current_tree,
                event,
                current_layout,
                adjusted_cursor,
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
        let Some(current_element) = self.current_element.as_ref() else {
            return mouse::Interaction::None;
        };
        let Some(current_layout) = layout
            .children()
            .next()
            .and_then(|wrapper| wrapper.children().next())
        else {
            return mouse::Interaction::None;
        };
        let current_offset =
            current_offset(self.mode, self.padding, &layout, state);
        let adjusted_cursor = translate_cursor(cursor, current_offset);
        current_element.as_widget().mouse_interaction(
            &state.current_tree,
            current_layout,
            adjusted_cursor,
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
        if self.current_element.is_none() {
            self.current_element = Some((self.view)(&state.current_value));
        }
        let current_element = self
            .current_element
            .as_mut()
            .expect("just materialized above");
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
        // The wrapper's bounds are `max(prev, cur)`, which can be
        // much smaller than the widget itself under Fill sizing.
        // `content_area` is the widget bounds minus padding — the
        // full region the slide runs across — and drives both the
        // slide displacement and the clip rect.
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
            None => 1.0,
        };

        let has_previous =
            state.previous_value.is_some() && state.previous_layout.is_some();

        let (cur_offset, prev_offset) = if has_previous {
            let t = transforms(self.mode, content_area.size(), progress);
            (t.current.translation, t.previous.translation)
        } else {
            (Vector::ZERO, Vector::ZERO)
        };

        renderer.with_layer(content_area, |renderer| {
            // Outgoing drawn first, incoming on top — where they
            // overlap mid-slide, the incoming wins z-order so it
            // reads as arriving rather than being covered by a slice
            // of the exiting content.
            if has_previous
                && let Some(prev_value) = state.previous_value.as_ref()
                && let Some(prev_node) = state.previous_layout.as_ref()
            {
                // Previous is re-materialized per draw rather than
                // stashed on `self`: it's decorative, doesn't receive
                // events, and renders in its widget's neutral
                // appearance — we don't carry widget-self state
                // across a swap boundary.
                let prev_element = (self.view)(prev_value);
                // Aligned within the content area, not the wrapper.
                // For matching alignment on both children, this
                // places the previous's canonical position exactly
                // on the current's — a shared anchor to slide from.
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
                renderer.with_translation(prev_offset, |renderer| {
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

            if let Some(current_element) = self.current_element.as_ref() {
                renderer.with_translation(cur_offset, |renderer| {
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
            }
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
        if self.current_element.is_none() {
            self.current_element = Some((self.view)(&state.current_value));
        }
        let element = self.current_element.as_mut()?;
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
