//! The [`TileGrid`] widget implementation.
//!
//! This module contains the main [`TileGrid`] widget, event types, and the
//! [`Catalog`] trait for theming.

use std::collections::HashMap;

use iced_widget::container;

use crate::core::keyboard;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay::{self, Group};
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    self, Animation, Background, Border, Color, Element, Event, Layout, Length,
    Pixels, Point, Rectangle, Shell, Size, Theme, Vector, Widget,
};

use super::content::Content;
use super::engine::{Internal, MoveMode};
use super::item_id::ItemId;
use super::state;

const DRAG_DEADBAND_DISTANCE: f32 = 10.0;
const RESIZE_LEEWAY: f32 = 8.0;

/// How the widget selects between [`MoveMode::Swap`] and
/// [`MoveMode::Place`] during drags.
///
/// - [`SwapMode::Auto`] (default): drags use [`MoveMode::Swap`],
///   unless the user holds `Shift` during the drag, which flips to
///   [`MoveMode::Place`] for that interaction.
/// - [`SwapMode::Never`]: drags always use [`MoveMode::Place`].
///   `Shift` has no effect.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SwapMode {
    /// Swap by default; `Shift` overrides to Place for that drag.
    #[default]
    Auto,
    /// Always Place; `Shift` has no effect.
    Never,
}

impl SwapMode {
    /// Resolves `(SwapMode, shift_held)` to the engine's [`MoveMode`].
    pub(crate) fn resolve(self, shift: bool) -> MoveMode {
        match (self, shift) {
            (Self::Auto, false) => MoveMode::Swap,
            (Self::Auto, true) => MoveMode::Place,
            (Self::Never, _) => MoveMode::Place,
        }
    }
}

/// How cell height is determined in the grid.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum CellHeight {
    /// Cells are square — height equals the computed cell width.
    #[default]
    Auto,
    /// Each row has a fixed pixel height.
    Fixed(f32),
}

/// The phase of a drag or resize interaction.
///
/// [`State::perform`](super::State::perform) defers engine mutations
/// during `Started`/`Ongoing` phases — the widget computes a visual
/// preview by cloning the engine. Only `Ended` commits the final
/// layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    /// The drag/resize interaction just began (first frame).
    Started,
    /// The drag/resize is in progress (subsequent frames).
    Ongoing,
    /// The drag/resize just ended (mouse/finger released).
    Ended,
}

/// An action produced by a [`TileGrid`] widget.
///
/// The widget emits these through its [`on_action`](TileGrid::on_action)
/// callback. The application should call
/// [`State::perform`](super::State::perform) to apply the action to the
/// grid state, optionally inspecting the action first (e.g. to track
/// focus on click).
///
/// # Example
///
/// ```ignore
/// Message::GridAction(action) => {
///     if action.is_click() {
///         self.focus = Some(action.id());
///     }
///     self.state.perform(action, |_, item| item.is_pinned);
/// }
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Action {
    /// An item was clicked.
    Click(ItemId),
    /// An item move operation (drag by title bar).
    Move {
        /// The item being moved.
        id: ItemId,
        /// The new column position.
        x: u16,
        /// The new row position.
        y: u16,
        /// The phase of the drag interaction.
        phase: DragPhase,
        /// The resolved engine [`MoveMode`] for this drag tick.
        /// Derived from the widget's [`SwapMode`] and the
        /// current keyboard modifier state.
        mode: MoveMode,
    },
    /// An item resize operation (drag by edge).
    Resize {
        /// The item being resized.
        id: ItemId,
        /// The new width in columns.
        w: u16,
        /// The new height in rows.
        h: u16,
        /// The phase of the resize interaction.
        phase: DragPhase,
    },
}

impl Action {
    /// Returns the [`ItemId`] of the item this action targets.
    #[must_use]
    pub fn id(&self) -> ItemId {
        match *self {
            Self::Click(id)
            | Self::Move { id, .. }
            | Self::Resize { id, .. } => id,
        }
    }

    /// Returns `true` if this is a click action.
    #[must_use]
    pub fn is_click(&self) -> bool {
        matches!(self, Self::Click(_))
    }

    /// Returns `true` if this is any move action.
    #[must_use]
    pub fn is_move(&self) -> bool {
        matches!(self, Self::Move { .. })
    }

    /// Returns `true` if this is any resize action.
    #[must_use]
    pub fn is_resize(&self) -> bool {
        matches!(self, Self::Resize { .. })
    }
}

/// A grid-based layout widget inspired by [GridStack.js](https://gridstackjs.com/).
///
/// Items are placed on a discrete grid with a fixed number of columns. Each item
/// occupies an integer-sized rectangle `(x, y, w, h)` in grid coordinates.
/// Users can drag items by their title bars to move them, or drag the bottom-right
/// edges to resize them.
///
/// Unlike [`PaneGrid`], which uses recursive binary splits, `TileGrid` uses
/// explicit integer coordinates. This allows arbitrary layouts including
/// L-shaped arrangements and items of varying sizes.
///
/// The widget does **not** mutate the engine state directly. Instead, it emits
/// [`Action`]s through a single callback that the application handles in its
/// `update` function by calling [`State::perform`](super::State::perform).
///
/// # Example
///
/// ```ignore
/// use sweeten::widget::tile_grid::{self, TileGrid, Content, TitleBar};
///
/// let grid = TileGrid::new(&state, |id, item| {
///     Content::new(text(&item.label))
///         .title_bar(TitleBar::new(text("Title")).padding(5))
/// })
/// .spacing(10)
/// .on_action(Message::GridAction);
/// ```
///
/// [`PaneGrid`]: https://docs.iced.rs/iced/widget/pane_grid/struct.PaneGrid.html
pub struct TileGrid<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    internal: &'a Internal,
    items: Vec<ItemId>,
    contents: Vec<Content<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
    spacing: f32,
    cell_height: CellHeight,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    locked: bool,
    /// Item IDs that are pinned/held and should not be displaced
    /// during drag preview collision resolution.
    held_ids: Vec<ItemId>,
    swap_mode: SwapMode,
    class: <Theme as Catalog>::Class<'a>,
    last_mouse_interaction: Option<mouse::Interaction>,
}

impl<'a, Message, Theme, Renderer> TileGrid<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    /// Creates a [`TileGrid`] with the given [`State`] and view function.
    ///
    /// The view function is called once for each item in the grid, receiving
    /// the item's [`ItemId`] and a reference to its user data.
    ///
    /// Prefer the [`tile_grid`](super::super::tile_grid) helper function.
    ///
    /// [`State`]: super::State
    pub(crate) fn new<T>(
        state: &'a state::State<T>,
        view: impl Fn(ItemId, &'a T) -> Content<'a, Message, Theme, Renderer>,
    ) -> Self {
        let items: Vec<ItemId> = state.iter().map(|(id, _)| id).collect();
        let contents: Vec<_> =
            state.iter().map(|(id, data)| view(id, data)).collect();

        // Derive held IDs from the Content builders — users express
        // held intent per-item via `Content::held(bool)`.
        let held_ids: Vec<ItemId> = items
            .iter()
            .zip(&contents)
            .filter_map(|(&id, content)| content.is_held().then_some(id))
            .collect();

        Self {
            internal: &state.internal,
            items,
            contents,
            width: Length::Fill,
            height: Length::Shrink,
            spacing: 0.0,
            cell_height: CellHeight::default(),
            on_action: None,
            locked: false,
            held_ids,
            swap_mode: SwapMode::default(),
            class: <Theme as Catalog>::default(),
            last_mouse_interaction: None,
        }
    }

    /// Sets the width of the [`TileGrid`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`TileGrid`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the spacing between grid items, in pixels.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets how cell height is computed.
    ///
    /// - [`CellHeight::Auto`] — cells are square (height = width)
    /// - [`CellHeight::Fixed`] — each row has a fixed pixel height
    pub fn cell_height(mut self, cell_height: CellHeight) -> Self {
        self.cell_height = cell_height;
        self
    }

    /// Sets the callback invoked when the widget produces an [`Action`].
    ///
    /// The single callback replaces separate click/move/resize callbacks.
    /// The application should call [`State::perform`](super::State::perform)
    /// to apply the action, optionally inspecting it first.
    pub fn on_action<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(Action) -> Message,
    {
        self.on_action = Some(Box::new(f));
        self
    }

    /// Sets the callback invoked when the widget produces an [`Action`],
    /// if `Some`.
    ///
    /// If `None`, all interactions will be disabled.
    pub fn on_action_maybe<F>(mut self, f: Option<F>) -> Self
    where
        F: 'a + Fn(Action) -> Message,
    {
        self.on_action = f.map(|f| Box::new(f) as _);
        self
    }

    /// Locks the grid, disabling all move and resize interactions.
    ///
    /// When locked, items cannot be dragged or resized. Click events
    /// are still emitted. Per-item control is available via
    /// [`Content::draggable`] and [`Content::resizable`].
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Sets the widget's swap-mode policy.
    ///
    /// - [`SwapMode::Auto`] (default): drags swap by default; holding
    ///   `Shift` during a drag flips to Place mode for that
    ///   interaction.
    /// - [`SwapMode::Never`]: drags always Place; Shift has no effect.
    #[must_use]
    pub fn swap_mode(mut self, swap_mode: SwapMode) -> Self {
        self.swap_mode = swap_mode;
        self
    }

    /// Sets the style of the [`TileGrid`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        <Theme as Catalog>::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Computes the cell dimensions from the given total width.
    fn cell_dimensions(&self, total_width: f32) -> (f32, f32) {
        let cols = f32::from(self.internal.columns());
        let cell_w = (total_width - (cols - 1.0) * self.spacing) / cols;
        let cell_h = match self.cell_height {
            CellHeight::Auto => cell_w,
            CellHeight::Fixed(h) => h,
        };
        (cell_w, cell_h)
    }

    /// Computes the total grid height from the engine state and cell dimensions.
    fn grid_height(&self, cell_h: f32) -> f32 {
        let rows = f32::from(self.internal.get_row());
        if rows == 0.0 {
            0.0
        } else {
            rows * cell_h + (rows - 1.0) * self.spacing
        }
    }

    /// Converts a pixel rectangle from `item_regions`-style tuple to a [`Rectangle`].
    fn pixel_rect(region: (f32, f32, f32, f32)) -> Rectangle {
        Rectangle {
            x: region.0,
            y: region.1,
            width: region.2,
            height: region.3,
        }
    }
}

/// Ephemeral interaction state stored in the widget tree.
#[derive(Debug, Default, Clone)]
enum Interaction {
    /// No interaction in progress.
    #[default]
    Idle,
    /// The user is dragging an item to move it.
    Moving {
        /// The item being moved.
        id: ItemId,
        /// The cursor position when the drag started.
        origin: Point,
        /// Offset from the item's top-left corner to the grab point.
        /// Used to render the item freely under the cursor during drag.
        grab_offset: Vector,
        /// The item's grid position when the drag started.
        start_x: u16,
        start_y: u16,
        /// Cell dimensions at drag start (for pixel-to-grid conversion).
        cell_w: f32,
        cell_h: f32,
        /// Whether a `DragPhase::Started` event has already been emitted.
        started: bool,
    },
    /// The user is dragging an edge to resize an item.
    Resizing {
        /// The item being resized.
        id: ItemId,
        /// The cursor position when the drag started.
        origin: Point,
        /// The item's grid size when the drag started.
        start_w: u16,
        start_h: u16,
        /// Cell dimensions at drag start.
        cell_w: f32,
        cell_h: f32,
        /// Whether a `DragPhase::Started` event has already been emitted.
        started: bool,
    },
}

/// The drag target + resolved engine mode for an active drag.
#[derive(Debug, Clone, Copy)]
struct DragTarget {
    id: ItemId,
    x: u16,
    y: u16,
    mode: MoveMode,
}

/// The resize target during an active resize. When set, the layout
/// method clones the engine and applies this resize to compute a
/// preview — the committed engine state is untouched.
#[derive(Debug, Clone, Copy)]
struct ResizeTarget {
    id: ItemId,
    w: u16,
    h: u16,
}

#[derive(Default)]
struct Memory {
    interaction: Interaction,
    order: Vec<ItemId>,
    last_hovered: Option<usize>,
    animations: ItemAnimations,
    /// Tentative grid target during an active drag. When set, the
    /// layout method clones the engine and applies this move to
    /// compute a preview — the committed engine state is untouched.
    drag_target: Option<DragTarget>,
    /// Tentative resize dimensions during an active resize. When set,
    /// the layout method clones the engine and applies this resize to
    /// compute a preview — the committed engine state is untouched.
    resize_target: Option<ResizeTarget>,
    /// Most recently seen keyboard modifiers. Kept in sync by the
    /// `ModifiersChanged` event; read during drag to decide whether
    /// Shift is forcing Place mode.
    modifiers: keyboard::Modifiers,
}

/// Per-item position animations for smooth transitions.
#[derive(Debug, Clone)]
struct ItemAnimations {
    /// Animated X offset for each item (pixels, from layout position).
    offsets_x: HashMap<ItemId, Animation<f32>>,
    /// Animated Y offset for each item (pixels, from layout position).
    offsets_y: HashMap<ItemId, Animation<f32>>,
    /// Last known pixel position for each item (top-left corner).
    last_positions: HashMap<ItemId, (f32, f32)>,
    /// Ghost placeholder opacity animation.
    ghost_opacity: Animation<bool>,
    /// The item that was being dragged when the ghost was last shown.
    ghost_item: Option<ItemId>,
    /// Last known ghost snap position (top-left pixel coords).
    ghost_last_pos: Option<(f32, f32)>,
    /// Animated X offset for the ghost (pixels, from snap position).
    ghost_offset_x: Animation<f32>,
    /// Animated Y offset for the ghost (pixels, from snap position).
    ghost_offset_y: Animation<f32>,
    /// Current time instant for interpolation.
    now: Option<Instant>,
}

impl Default for ItemAnimations {
    fn default() -> Self {
        Self {
            offsets_x: HashMap::new(),
            offsets_y: HashMap::new(),
            last_positions: HashMap::new(),
            ghost_opacity: Animation::new(false),
            ghost_item: None,
            ghost_last_pos: None,
            ghost_offset_x: Animation::new(0.0),
            ghost_offset_y: Animation::new(0.0),
            now: None,
        }
    }
}

impl ItemAnimations {
    fn new_animation(value: f32) -> Animation<f32> {
        Animation::new(value)
            .quick()
            .easing(crate::core::animation::Easing::EaseOut)
    }

    fn new_ghost_animation(value: bool) -> Animation<bool> {
        Animation::new(value)
            .quick()
            .easing(crate::core::animation::Easing::EaseOut)
    }

    /// Returns true if any item animation is in progress.
    fn is_animating(&self, now: Instant) -> bool {
        self.offsets_x.values().any(|anim| anim.is_animating(now))
            || self.offsets_y.values().any(|anim| anim.is_animating(now))
            || self.ghost_opacity.is_animating(now)
            || self.ghost_offset_x.is_animating(now)
            || self.ghost_offset_y.is_animating(now)
    }

    /// Update animations based on new item positions from layout.
    /// `regions` maps ItemId -> (px_x, px_y, pw, ph).
    /// `dragged_id` is the item currently being dragged (should not be animated).
    #[allow(clippy::type_complexity)]
    fn update_positions(
        &mut self,
        regions: &[(ItemId, (f32, f32, f32, f32))],
        dragged_id: Option<ItemId>,
        now: Instant,
    ) {
        for &(id, (px, py, _, _)) in regions {
            // Don't animate the item being dragged
            if dragged_id == Some(id) {
                self.last_positions.insert(id, (px, py));
                continue;
            }

            if let Some(&(old_x, old_y)) = self.last_positions.get(&id) {
                let dx = old_x - px;
                let dy = old_y - py;

                // Only start a new animation if the position actually changed
                // by a meaningful amount (> 0.5px to avoid float noise).
                if dx.abs() > 0.5 || dy.abs() > 0.5 {
                    let anim_x = Self::new_animation(dx).go(0.0, now);
                    let anim_y = Self::new_animation(dy).go(0.0, now);
                    self.offsets_x.insert(id, anim_x);
                    self.offsets_y.insert(id, anim_y);
                }
            }

            self.last_positions.insert(id, (px, py));
        }

        // Clean up stale entries for items that no longer exist.
        let current_ids: std::collections::HashSet<ItemId> =
            regions.iter().map(|(id, _)| *id).collect();
        self.offsets_x.retain(|id, _| current_ids.contains(id));
        self.offsets_y.retain(|id, _| current_ids.contains(id));
        self.last_positions.retain(|id, _| current_ids.contains(id));
    }

    /// Get the current interpolated offset for an item.
    fn get_offset(&self, id: ItemId, now: Instant) -> Vector {
        let x = self
            .offsets_x
            .get(&id)
            .filter(|anim| anim.is_animating(now))
            .map(|anim| anim.interpolate_with(|v| v, now))
            .unwrap_or(0.0);
        let y = self
            .offsets_y
            .get(&id)
            .filter(|anim| anim.is_animating(now))
            .map(|anim| anim.interpolate_with(|v| v, now))
            .unwrap_or(0.0);
        Vector::new(x, y)
    }

    /// Start the ghost fade-in animation.
    fn show_ghost(&mut self, id: ItemId, now: Instant) {
        if self.ghost_item != Some(id) {
            self.ghost_opacity = Self::new_ghost_animation(false).go(true, now);
            self.ghost_item = Some(id);
        }
    }

    /// Hide the ghost (reset state).
    fn hide_ghost(&mut self) {
        if self.ghost_item.is_some() {
            self.ghost_item = None;
            self.ghost_opacity = Self::new_ghost_animation(false);
            self.ghost_last_pos = None;
        }
    }

    /// Get the current ghost opacity (0.0 to 1.0).
    fn ghost_alpha(&self, now: Instant) -> f32 {
        self.ghost_opacity.interpolate(0.0, 1.0, now)
    }

    /// Update ghost position animation when the snap position changes.
    fn update_ghost_position(&mut self, target: Rectangle, now: Instant) {
        let new_pos = (target.x, target.y);

        if let Some((old_x, old_y)) = self.ghost_last_pos {
            let dx = old_x - new_pos.0;
            let dy = old_y - new_pos.1;

            if dx.abs() > 0.5 || dy.abs() > 0.5 {
                self.ghost_offset_x = Self::new_animation(dx).go(0.0, now);
                self.ghost_offset_y = Self::new_animation(dy).go(0.0, now);
            }
        }

        self.ghost_last_pos = Some(new_pos);
    }

    /// Get the current interpolated offset for the ghost.
    fn ghost_offset(&self, now: Instant) -> Vector {
        let x = if self.ghost_offset_x.is_animating(now) {
            self.ghost_offset_x.interpolate_with(|v| v, now)
        } else {
            0.0
        };
        let y = if self.ghost_offset_y.is_animating(now) {
            self.ghost_offset_y.interpolate_with(|v| v, now)
        } else {
            0.0
        };
        Vector::new(x, y)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TileGrid<'_, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Memory>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Memory::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.contents.iter().map(Content::state).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let Memory { order, .. } = tree.state.downcast_ref();

        // ItemId is monotonically increasing and iterated by Ord (BTreeMap),
        // so new states always appear at the end. We can remove states for
        // items that no longer exist, then diff_children_custom will reconcile.
        let mut i = 0;
        let mut j = 0;
        tree.children.retain(|_| {
            let retain = self.items.get(i) == order.get(j);
            if retain {
                i += 1;
            }
            j += 1;
            retain
        });

        tree.diff_children_custom(
            &self.contents,
            |state, content| content.diff(state),
            Content::state,
        );

        let Memory { order, .. } = tree.state.downcast_mut();
        order.clone_from(&self.items);
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
        let max = limits.max();
        // Resolve the width first. The height depends on cell dimensions
        // which depend on width.
        let resolved_width = match self.width {
            Length::Fill | Length::FillPortion(_) => max.width,
            Length::Shrink => max.width, // grid fills available width
            Length::Fixed(w) => w.min(max.width),
        };

        let (cell_w, cell_h) = self.cell_dimensions(resolved_width);
        let grid_h = self.grid_height(cell_h);

        let resolved_height = match self.height {
            Length::Fill | Length::FillPortion(_) => max.height,
            Length::Shrink => grid_h.min(max.height),
            Length::Fixed(h) => h.min(max.height),
        };

        let bounds = Size::new(resolved_width, resolved_height);

        // Update animation state with the new positions.
        let memory = tree.state.downcast_mut::<Memory>();

        // Compute item regions. During an active drag, clone the engine
        // and apply the tentative move so we get a *preview* layout
        // without modifying the committed engine state.
        let regions = if let Some(target) = memory.drag_target {
            // The preview must match the drop exactly: no batch mode
            // (so gravity runs), and the mover is NOT added to held
            // (so it can float up to fill gaps). Otherwise the ghost
            // shows a pre-gravity position and the drop teleports
            // the item elsewhere.
            let mut preview = self.internal.clone();
            preview.save_snapshot();
            preview.move_item_held(
                target.id,
                target.x,
                target.y,
                &self.held_ids,
                target.mode,
            );
            Self::compute_regions_for(
                &preview,
                resolved_width,
                cell_w,
                cell_h,
                self.spacing,
            )
        } else if let Some(resize) = memory.resize_target {
            // No save_snapshot() needed: resize_item_held uses
            // fix_collisions → try_swap_pack, which early-returns
            // when there's no snapshot. Swap-pack is only relevant
            // for moves, not resizes.
            let mut preview = self.internal.clone();
            preview.resize_item_held(
                resize.id,
                resize.w,
                resize.h,
                &self.held_ids,
            );
            Self::compute_regions_for(
                &preview,
                resolved_width,
                cell_w,
                cell_h,
                self.spacing,
            )
        } else {
            self.compute_regions(resolved_width, cell_w, cell_h)
        };
        let dragged_id = match &memory.interaction {
            Interaction::Moving { id, .. }
            | Interaction::Resizing { id, .. } => Some(*id),
            Interaction::Idle => None,
        };
        let now = memory.animations.now.unwrap_or_else(Instant::now);
        memory
            .animations
            .update_positions(&regions, dragged_id, now);

        if let Some(dragged) = dragged_id
            && let Some(&(_, (px, py, pw, ph))) =
                regions.iter().find(|(id, _)| *id == dragged)
        {
            memory.animations.update_ghost_position(
                Rectangle::new(Point::new(px, py), Size::new(pw, ph)),
                now,
            );
        }

        let children = self
            .items
            .iter()
            .zip(&mut self.contents)
            .zip(tree.children.iter_mut())
            .map(|((id, content), tree)| {
                let region =
                    regions.iter().find(|(rid, _)| rid == id).map(|(_, r)| *r);

                if let Some(region) = region {
                    let rect = Self::pixel_rect(region);
                    let size = Size::new(rect.width, rect.height);

                    let node = content.layout(
                        tree,
                        renderer,
                        &layout::Limits::new(size, size),
                    );
                    node.move_to(Point::new(rect.x, rect.y))
                } else {
                    layout::Node::new(Size::ZERO)
                }
            })
            .collect();

        layout::Node::with_children(bounds, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.contents
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((content, state), layout)| {
                    content.operate(state, layout, renderer, operation);
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
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Memory { interaction, .. } = tree.state.downcast_mut();

        let picked_item = match interaction {
            Interaction::Moving { id, .. }
            | Interaction::Resizing { id, .. } => Some(*id),
            Interaction::Idle => None,
        };

        // Propagate events to contents
        for ((id, content), (tree, layout)) in self
            .items
            .iter()
            .copied()
            .zip(&mut self.contents)
            .zip(tree.children.iter_mut().zip(layout.children()))
        {
            let is_picked = picked_item == Some(id);
            content.update(
                tree, event, layout, cursor, renderer, shell, viewport,
                is_picked,
            );
        }

        let bounds = layout.bounds();
        let Memory {
            interaction,
            animations,
            drag_target,
            resize_target,
            modifiers,
            ..
        } = tree.state.downcast_mut();

        match event {
            // Keep modifiers fresh — other branches read them to
            // decide whether Shift is forcing Place mode.
            Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                *modifiers = *m;
                if let Some(target) = drag_target {
                    target.mode = self.swap_mode.resolve(m.shift());
                    shell.request_redraw();
                }
            }
            // Reset modifiers on window unfocus — we may have missed
            // release events while the window was backgrounded.
            Event::Window(window::Event::Unfocused) => {
                *modifiers = keyboard::Modifiers::empty();
                if let Some(target) = drag_target {
                    target.mode = self.swap_mode.resolve(false);
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                animations.now = Some(*now);

                // Request another frame if animations are still in progress.
                if animations.is_animating(*now) {
                    shell.request_redraw();
                }

                // Manage ghost animation based on drag state.
                match interaction {
                    Interaction::Moving { id, origin, .. } => {
                        if cursor.position().is_some_and(|pos| {
                            pos.distance(*origin) > DRAG_DEADBAND_DISTANCE
                        }) {
                            animations.show_ghost(*id, *now);
                        }
                    }
                    Interaction::Idle | Interaction::Resizing { .. } => {
                        animations.hide_ghost();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor_position) = cursor.position_over(bounds) {
                    shell.capture_event();

                    let (cell_w, cell_h) = self.cell_dimensions(bounds.width);

                    // Check for resize first (bottom-right edge detection)
                    if self.on_action.is_some()
                        && !self.locked
                        && let Some(resize_id) = self.find_resize_target(
                            layout,
                            cursor_position,
                            bounds,
                        )
                        && let Some(item) = self.internal.get(resize_id)
                    {
                        *interaction = Interaction::Resizing {
                            id: resize_id,
                            origin: cursor_position,
                            start_w: item.w,
                            start_h: item.h,
                            cell_w,
                            cell_h,
                            started: false,
                        };
                        return;
                    }

                    // Check for click/drag on an item
                    self.click_item(
                        interaction,
                        layout,
                        cursor_position,
                        shell,
                        cell_w,
                        cell_h,
                    );
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                match interaction {
                    Interaction::Moving {
                        id,
                        origin,
                        start_x,
                        start_y,
                        cell_w,
                        cell_h,
                        started,
                        ..
                    } if *started => {
                        if let Some(on_action) = &self.on_action
                            && let Some(cursor_position) = cursor.position()
                        {
                            let dx = cursor_position.x - origin.x;
                            let dy = cursor_position.y - origin.y;
                            let step_w = *cell_w + self.spacing;
                            let step_h = *cell_h + self.spacing;
                            let grid_dx = (dx / step_w).round() as i32;
                            let grid_dy = (dy / step_h).round() as i32;
                            let new_x =
                                (*start_x as i32 + grid_dx).max(0) as u16;
                            let new_y =
                                (*start_y as i32 + grid_dy).max(0) as u16;

                            // Resolve mode from the current drag
                            // target (which was kept in sync with
                            // modifiers on every CursorMoved /
                            // ModifiersChanged tick). Fall back to
                            // computing from live modifiers if no
                            // target was ever set.
                            let mode = drag_target
                                .map(|t| t.mode)
                                .unwrap_or_else(|| {
                                    self.swap_mode.resolve(modifiers.shift())
                                });

                            shell.publish(on_action(Action::Move {
                                id: *id,
                                x: new_x,
                                y: new_y,
                                phase: DragPhase::Ended,
                                mode,
                            }));
                        }
                    }
                    Interaction::Resizing {
                        id,
                        origin,
                        start_w,
                        start_h,
                        cell_w,
                        cell_h,
                        started,
                    } if *started => {
                        if let Some(on_action) = &self.on_action
                            && let Some(cursor_position) = cursor.position()
                        {
                            let dx = cursor_position.x - origin.x;
                            let dy = cursor_position.y - origin.y;
                            let step_w = *cell_w + self.spacing;
                            let step_h = *cell_h + self.spacing;
                            let grid_dw = (dx / step_w).round() as i32;
                            let grid_dh = (dy / step_h).round() as i32;
                            let new_w =
                                (*start_w as i32 + grid_dw).max(1) as u16;
                            let new_h =
                                (*start_h as i32 + grid_dh).max(1) as u16;

                            shell.publish(on_action(Action::Resize {
                                id: *id,
                                w: new_w,
                                h: new_h,
                                phase: DragPhase::Ended,
                            }));
                        }
                    }
                    _ => {}
                }
                *interaction = Interaction::Idle;
                *drag_target = None;
                *resize_target = None;
                animations.hide_ghost();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => {
                match interaction {
                    Interaction::Moving {
                        id,
                        origin,
                        start_x,
                        start_y,
                        cell_w,
                        cell_h,
                        started,
                        ..
                    } => {
                        if let Some(on_action) = &self.on_action
                            && let Some(cursor_position) = cursor.position()
                            && cursor_position.distance(*origin)
                                > DRAG_DEADBAND_DISTANCE
                        {
                            let dx = cursor_position.x - origin.x;
                            let dy = cursor_position.y - origin.y;
                            let step_w = *cell_w + self.spacing;
                            let step_h = *cell_h + self.spacing;
                            let grid_dx = (dx / step_w).round() as i32;
                            let grid_dy = (dy / step_h).round() as i32;
                            let new_x =
                                (*start_x as i32 + grid_dx).max(0) as u16;
                            let new_y =
                                (*start_y as i32 + grid_dy).max(0) as u16;

                            // Start ghost animation when drag begins
                            let now =
                                animations.now.unwrap_or_else(Instant::now);
                            animations.show_ghost(*id, now);

                            // Resolve the engine mode from the current
                            // swap policy + live modifier state.
                            let mode =
                                self.swap_mode.resolve(modifiers.shift());

                            // Store tentative grid position + mode for
                            // the preview layout (computed in layout()).
                            *drag_target = Some(DragTarget {
                                id: *id,
                                x: new_x,
                                y: new_y,
                                mode,
                            });

                            let phase = if *started {
                                DragPhase::Ongoing
                            } else {
                                *started = true;
                                DragPhase::Started
                            };

                            shell.publish(on_action(Action::Move {
                                id: *id,
                                x: new_x,
                                y: new_y,
                                phase,
                                mode,
                            }));
                        }
                        shell.request_redraw();
                    }
                    Interaction::Resizing {
                        id,
                        origin,
                        start_w,
                        start_h,
                        cell_w,
                        cell_h,
                        started,
                    } => {
                        if let Some(on_action) = &self.on_action
                            && let Some(cursor_position) = cursor.position()
                        {
                            let dx = cursor_position.x - origin.x;
                            let dy = cursor_position.y - origin.y;
                            let step_w = *cell_w + self.spacing;
                            let step_h = *cell_h + self.spacing;
                            let grid_dw = (dx / step_w).round() as i32;
                            let grid_dh = (dy / step_h).round() as i32;
                            let new_w =
                                (*start_w as i32 + grid_dw).max(1) as u16;
                            let new_h =
                                (*start_h as i32 + grid_dh).max(1) as u16;

                            // Store tentative size for the preview
                            // layout (computed in layout()).
                            *resize_target = Some(ResizeTarget {
                                id: *id,
                                w: new_w,
                                h: new_h,
                            });

                            let phase = if *started {
                                DragPhase::Ongoing
                            } else {
                                *started = true;
                                DragPhase::Started
                            };

                            shell.publish(on_action(Action::Resize {
                                id: *id,
                                w: new_w,
                                h: new_h,
                                phase,
                            }));
                        }
                        shell.request_redraw();
                    }
                    Interaction::Idle => {}
                }
            }
            _ => {}
        }

        // Track which item the cursor hovers so we can request redraws when it
        // changes.  This keeps show/hide of title-bar controls in sync.
        {
            let hovered_index = cursor.position_over(bounds).and_then(|pos| {
                layout
                    .children()
                    .enumerate()
                    .find(|(_, child)| child.bounds().contains(pos))
                    .map(|(i, _)| i)
            });

            let memory = tree.state.downcast_mut::<Memory>();
            if memory.last_hovered != hovered_index {
                memory.last_hovered = hovered_index;
                shell.request_redraw();
            }
        }

        // Detect mouse interaction changes (cursor type) so we request redraws
        // for hover effects like the resize cursor or grab cursor.
        if shell.redraw_request() != window::RedrawRequest::NextFrame {
            let current_interaction =
                &tree.state.downcast_ref::<Memory>().interaction;

            let interaction = self
                .grid_interaction(current_interaction, layout, cursor)
                .or_else(|| {
                    let drag_enabled = self.on_action.is_some() && !self.locked;
                    self.items
                        .iter()
                        .zip(&self.contents)
                        .zip(layout.children())
                        .find_map(|((_, content), layout)| {
                            content.grid_interaction(
                                layout,
                                cursor,
                                drag_enabled,
                            )
                        })
                })
                .unwrap_or(mouse::Interaction::None);

            if let Event::Window(window::Event::RedrawRequested(_)) = event {
                self.last_mouse_interaction = Some(interaction);
            } else if self
                .last_mouse_interaction
                .is_some_and(|last| last != interaction)
            {
                shell.request_redraw();
            }
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
        let Memory {
            interaction: current_interaction,
            ..
        } = tree.state.downcast_ref();

        match current_interaction {
            Interaction::Moving { origin, .. } => {
                if let Some(pos) = cursor.position()
                    && pos.distance(*origin) > DRAG_DEADBAND_DISTANCE
                {
                    return mouse::Interaction::Grabbing;
                }
                return mouse::Interaction::Grab;
            }
            Interaction::Resizing { .. } => {
                return mouse::Interaction::ResizingDiagonallyDown;
            }
            Interaction::Idle => {}
        }

        let bounds = layout.bounds();

        // Check for resize cursor on edges
        if self.on_action.is_some()
            && !self.locked
            && let Some(cursor_position) = cursor.position_over(bounds)
            && self
                .find_resize_target(layout, cursor_position, bounds)
                .is_some()
        {
            return mouse::Interaction::ResizingDiagonallyDown;
        }

        // Check content interactions
        let drag_enabled = self.on_action.is_some() && !self.locked;

        self.items
            .iter()
            .zip(&self.contents)
            .zip(&tree.children)
            .zip(layout.children())
            .map(|(((_, content), tree), layout)| {
                content.mouse_interaction(
                    tree,
                    layout,
                    cursor,
                    viewport,
                    renderer,
                    drag_enabled,
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
        let Memory {
            interaction,
            animations,
            drag_target,
            ..
        } = tree.state.downcast_ref();
        let now = animations.now.unwrap_or_else(Instant::now);

        let picked_item = match interaction {
            Interaction::Moving {
                id,
                origin,
                grab_offset,
                ..
            } => cursor
                .position()
                .filter(|pos| pos.distance(*origin) > DRAG_DEADBAND_DISTANCE)
                .map(|_| (*id, *grab_offset)),
            _ => None,
        };

        let resizing_id = match interaction {
            Interaction::Resizing { id, .. } => Some(*id),
            _ => None,
        };

        let item_cursor = if picked_item.is_some() {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };

        let mut render_picked = None;

        for (((id, content), tree), item_layout) in self
            .items
            .iter()
            .copied()
            .zip(&self.contents)
            .zip(&tree.children)
            .zip(layout.children())
        {
            match picked_item {
                Some((dragging, grab_offset)) if id == dragging => {
                    render_picked =
                        Some(((content, tree), item_layout, grab_offset));
                }
                _ => {
                    // When resizing an item, force its controls to stay
                    // visible by providing a cursor over its bounds.
                    let draw_cursor = if resizing_id == Some(id) {
                        let b = item_layout.bounds();
                        mouse::Cursor::Available(Point::new(
                            b.x + b.width / 2.0,
                            b.y + b.height / 2.0,
                        ))
                    } else {
                        item_cursor
                    };

                    // Apply animated offset for smooth transitions.
                    let offset = animations.get_offset(id, now);

                    if offset.x != 0.0 || offset.y != 0.0 {
                        renderer.with_translation(offset, |renderer| {
                            content.draw(
                                tree,
                                renderer,
                                theme,
                                defaults,
                                item_layout,
                                draw_cursor,
                                viewport,
                            );
                        });
                    } else {
                        content.draw(
                            tree,
                            renderer,
                            theme,
                            defaults,
                            item_layout,
                            draw_cursor,
                            viewport,
                        );
                    }
                }
            }
        }

        // Draw resize grip indicator on hovered, resizable items when
        // resize is enabled and the style provides a grip appearance.
        let grid_style = Catalog::style(theme, &self.class);

        if self.on_action.is_some()
            && !self.locked
            && let Some(ref grip) = grid_style.resize_grip
            && let Some(cursor_position) = item_cursor.position()
        {
            for ((id, content), item_layout) in self
                .items
                .iter()
                .copied()
                .zip(&self.contents)
                .zip(layout.children())
            {
                let item_bounds = item_layout.bounds();
                if resizing_id.is_none_or(|rid| rid != id)
                    && !item_bounds.contains(cursor_position)
                {
                    continue;
                }
                if !content.is_resizable() {
                    continue;
                }
                if picked_item.is_some_and(|(pid, _)| pid == id) {
                    continue;
                }

                // Draw a triangular grip pattern at the bottom-right:
                //       .
                //     . .
                //   . . .
                let margin = 6.0;
                let gap = 4.0;
                let anchor_x = item_bounds.x + item_bounds.width - margin;
                let anchor_y = item_bounds.y + item_bounds.height - margin;

                for row in 0..3_u8 {
                    for col in 0..=row {
                        let x = anchor_x - (col as f32) * gap;
                        let y = anchor_y - ((2 - row) as f32) * gap;

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x,
                                    y,
                                    width: grip.dot_size,
                                    height: grip.dot_size,
                                },
                                border: Border {
                                    radius: (grip.dot_size / 2.0).into(),
                                    ..Border::default()
                                },
                                ..renderer::Quad::default()
                            },
                            Background::Color(grip.color),
                        );
                    }
                }
            }
        }

        // Render the picked item last, floating freely under the cursor.
        // The item follows the mouse exactly (organic feel) while the engine
        // handles snapped grid positions for other items underneath.
        if let Some(((content, tree), item_layout, grab_offset)) = render_picked
            && let Some(cursor_position) = cursor.position()
        {
            // Draw a translucent ghost rectangle at the engine's snap position
            // (where the item would land if released now), with animated opacity.
            // The ghost slides smoothly between snap positions via an offset
            // animation, and is clipped to avoid overlapping items that are
            // still animating away from the ghost area.
            let snap_bounds = item_layout.bounds();
            let ghost_pos_offset = animations.ghost_offset(now);
            let ghost_target = Rectangle {
                x: snap_bounds.x + ghost_pos_offset.x,
                y: snap_bounds.y + ghost_pos_offset.y,
                ..snap_bounds
            };
            let ghost_alpha = animations.ghost_alpha(now);

            let dragged_id = picked_item.unwrap().0;
            let animating: Vec<(Rectangle, Vector)> = self
                .items
                .iter()
                .copied()
                .zip(layout.children())
                .filter_map(|(id, child_layout)| {
                    if id == dragged_id {
                        return None;
                    }
                    let offset = animations.get_offset(id, now);
                    if offset.x == 0.0 && offset.y == 0.0 {
                        return None;
                    }
                    Some((child_layout.bounds(), offset))
                })
                .collect();

            let ghost_bounds =
                clip_ghost_for_animating_items(ghost_target, &animating);

            // Pick the highlight for the current drag mode. When
            // dragging in Place (Shift held, or SwapMode::Never), use
            // `place_region` if set. Otherwise fall back to
            // `hovered_region`.
            let highlight = match drag_target.map(|t| t.mode) {
                Some(MoveMode::Place) => grid_style
                    .place_region
                    .as_ref()
                    .unwrap_or(&grid_style.hovered_region),
                _ => &grid_style.hovered_region,
            };

            renderer.fill_quad(
                renderer::Quad {
                    bounds: ghost_bounds,
                    border: Border {
                        color: highlight.border.color.scale_alpha(ghost_alpha),
                        ..highlight.border
                    },
                    ..renderer::Quad::default()
                },
                highlight.background.scale_alpha(ghost_alpha),
            );

            // Draw the floating item under the cursor.
            // Use the un-offset snap_bounds for translation so the floating
            // item follows the cursor freely regardless of ghost animation.
            let layout_pos = snap_bounds.position();
            let target = Point::new(
                cursor_position.x - grab_offset.x,
                cursor_position.y - grab_offset.y,
            );
            let translation =
                Vector::new(target.x - layout_pos.x, target.y - layout_pos.y);

            renderer.with_translation(translation, |renderer| {
                renderer.with_layer(snap_bounds, |renderer| {
                    content.draw(
                        tree,
                        renderer,
                        theme,
                        defaults,
                        item_layout,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                });
            });
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
        let children = self
            .contents
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((content, state), layout)| {
                content.overlay(state, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!children.is_empty()).then(|| Group::with_children(children).overlay())
    }
}

impl<'a, Message, Theme, Renderer> TileGrid<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    /// Compute item regions using the cell_height setting.
    fn compute_regions(
        &self,
        total_width: f32,
        cell_w: f32,
        cell_h: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        Self::compute_regions_for(
            self.internal,
            total_width,
            cell_w,
            cell_h,
            self.spacing,
        )
    }

    /// Compute pixel regions for the items in an arbitrary engine.
    fn compute_regions_for(
        internal: &Internal,
        total_width: f32,
        cell_w: f32,
        cell_h: f32,
        spacing: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        internal
            .items()
            .map(|item| {
                let x = f32::from(item.x);
                let y = f32::from(item.y);
                let w = f32::from(item.w);
                let h = f32::from(item.h);

                let px = (x * cell_w + x * spacing).round();
                let py = (y * cell_h + y * spacing).round();
                let pw = (w * cell_w + (w - 1.0) * spacing).round();
                let ph = (h * cell_h + (h - 1.0) * spacing).round();

                // Clamp width to not exceed total_width
                let pw = pw.min(total_width - px);

                (item.id, (px, py, pw, ph))
            })
            .collect()
    }

    /// Find an item whose right or bottom edge is near the cursor for resizing.
    fn find_resize_target(
        &self,
        _layout: Layout<'_>,
        cursor_position: Point,
        bounds: Rectangle,
    ) -> Option<ItemId> {
        let (cell_w, cell_h) = self.cell_dimensions(bounds.width);
        let regions = self.compute_regions(bounds.width, cell_w, cell_h);

        for (id, region) in &regions {
            // Look up the content for this item to check resizability.
            let content_idx =
                self.items.iter().position(|item_id| item_id == id);
            if let Some(idx) = content_idx
                && !self.contents[idx].is_resizable()
            {
                continue;
            }

            let rect = Self::pixel_rect(*region);
            // Translate to absolute coordinates
            let abs_rect = Rectangle {
                x: rect.x + bounds.x,
                y: rect.y + bounds.y,
                width: rect.width,
                height: rect.height,
            };

            let near_right =
                (cursor_position.x - (abs_rect.x + abs_rect.width)).abs()
                    < RESIZE_LEEWAY
                    && cursor_position.y >= abs_rect.y
                    && cursor_position.y <= abs_rect.y + abs_rect.height;

            let near_bottom =
                (cursor_position.y - (abs_rect.y + abs_rect.height)).abs()
                    < RESIZE_LEEWAY
                    && cursor_position.x >= abs_rect.x
                    && cursor_position.x <= abs_rect.x + abs_rect.width;

            if near_right || near_bottom {
                return Some(*id);
            }
        }

        None
    }

    /// Computes the current mouse interaction based on the action state and
    /// cursor position, without consulting child widgets.
    fn grid_interaction(
        &self,
        current_interaction: &Interaction,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) -> Option<mouse::Interaction> {
        match current_interaction {
            Interaction::Moving { origin, .. } => {
                if cursor.position().is_some_and(|pos| {
                    pos.distance(*origin) > DRAG_DEADBAND_DISTANCE
                }) {
                    return Some(mouse::Interaction::Grabbing);
                }
                return Some(mouse::Interaction::Grab);
            }
            Interaction::Resizing { .. } => {
                return Some(mouse::Interaction::ResizingDiagonallyDown);
            }
            Interaction::Idle => {}
        }

        let bounds = layout.bounds();

        if self.on_action.is_some()
            && !self.locked
            && let Some(cursor_position) = cursor.position_over(bounds)
            && self
                .find_resize_target(layout, cursor_position, bounds)
                .is_some()
        {
            return Some(mouse::Interaction::ResizingDiagonallyDown);
        }

        None
    }

    /// Handles a click on an item, potentially starting a drag.
    fn click_item(
        &self,
        interaction: &mut Interaction,
        layout: Layout<'_>,
        cursor_position: Point,
        shell: &mut Shell<'_, Message>,
        cell_w: f32,
        cell_h: f32,
    ) {
        let clicked = self
            .items
            .iter()
            .copied()
            .zip(&self.contents)
            .zip(layout.children())
            .find(|((_, _), layout)| layout.bounds().contains(cursor_position));

        if let Some(((id, content), item_layout)) = clicked {
            if let Some(on_action) = &self.on_action {
                shell.publish(on_action(Action::Click(id)));
            }

            if self.on_action.is_some()
                && !self.locked
                && content.is_draggable()
                && content.can_be_dragged_at(item_layout, cursor_position)
                && let Some(item) = self.internal.get(id)
            {
                let item_pos = item_layout.bounds().position();
                let grab_offset = Vector::new(
                    cursor_position.x - item_pos.x,
                    cursor_position.y - item_pos.y,
                );

                *interaction = Interaction::Moving {
                    id,
                    origin: cursor_position,
                    grab_offset,
                    start_x: item.x,
                    start_y: item.y,
                    cell_w,
                    cell_h,
                    started: false,
                };
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<TileGrid<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: core::Renderer + 'a,
{
    fn from(tile_grid: TileGrid<'a, Message, Theme, Renderer>) -> Self {
        Element::new(tile_grid)
    }
}

/// The appearance of a [`TileGrid`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the grid area.
    pub background: Option<Background>,
    /// The [`Border`] around the grid area.
    pub border: Border,
    /// The highlight shown when an item is being dragged over another
    /// under [`MoveMode::Swap`] (or when no [`place_region`] is set).
    ///
    /// [`place_region`]: Self::place_region
    pub hovered_region: Highlight,
    /// The highlight shown when the active drag is in
    /// [`MoveMode::Place`] (e.g. `Shift` is held, or `SwapMode::Never`
    /// is set). When `None`, the ghost uses [`hovered_region`] for
    /// both modes.
    ///
    /// [`hovered_region`]: Self::hovered_region
    pub place_region: Option<Highlight>,
    /// The appearance of the resize grip indicator shown on hovered items.
    ///
    /// Set to `None` to disable the resize grip.
    pub resize_grip: Option<ResizeGrip>,
}

/// A highlight region appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Highlight {
    /// The [`Background`] of the highlighted region.
    pub background: Background,
    /// The [`Border`] of the highlighted region.
    pub border: Border,
}

/// The appearance of the resize grip indicator drawn at the bottom-right
/// corner of a hovered grid item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeGrip {
    /// The color of the grip dots.
    pub color: Color,
    /// The size of each dot in pixels.
    pub dot_size: f32,
}

/// The theme catalog for a [`TileGrid`].
pub trait Catalog: container::Catalog {
    /// The item class of this [`Catalog`].
    type Class<'a>;

    /// The default class produced by this [`Catalog`].
    fn default<'a>() -> <Self as Catalog>::Class<'a>;

    /// The [`Style`] of a class.
    fn style(&self, class: &<Self as Catalog>::Class<'_>) -> Style;
}

/// A styling function for a [`TileGrid`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default_style)
    }

    fn style(&self, class: &StyleFn<'_, Self>) -> Style {
        class(self)
    }
}

/// Clips a ghost rectangle to avoid overlapping with items that are
/// animating away from the ghost's area.
///
/// Each entry in `animating_items` is `(layout_bounds, animation_offset)`,
/// where `layout_bounds` is the item's final (layout) position, and
/// `animation_offset` is the current visual offset from that position (the
/// offset starts large and converges to zero as the animation completes).
///
/// The returned rectangle is the ghost shrunk so it never visually overlaps
/// any of the animating items.
fn clip_ghost_for_animating_items(
    ghost: Rectangle,
    animating_items: &[(Rectangle, Vector)],
) -> Rectangle {
    let mut bounds = ghost;

    for &(layout_rect, offset) in animating_items {
        if offset.x == 0.0 && offset.y == 0.0 {
            continue;
        }

        // The item's visual rectangle: layout position + current offset.
        let visual = Rectangle {
            x: layout_rect.x + offset.x,
            y: layout_rect.y + offset.y,
            width: layout_rect.width,
            height: layout_rect.height,
        };

        // Skip if no overlap between current ghost bounds and this item.
        if bounds.y >= visual.y + visual.height
            || bounds.y + bounds.height <= visual.y
            || bounds.x >= visual.x + visual.width
            || bounds.x + bounds.width <= visual.x
        {
            continue;
        }

        // Clip vertically based on offset direction.
        if offset.y < 0.0 {
            // Item visual is above its layout position, sliding DOWN.
            // It occupies space in the lower portion of the ghost area.
            // Reveal the ghost from the top: clip the bottom edge.
            let available = (visual.y - bounds.y).max(0.0);
            bounds.height = bounds.height.min(available);
        } else if offset.y > 0.0 {
            // Item visual is below its layout position, sliding UP.
            // It occupies space in the upper portion of the ghost area.
            // Reveal the ghost from the bottom: clip the top edge.
            let new_top = (visual.y + visual.height)
                .max(bounds.y)
                .min(bounds.y + bounds.height);
            bounds.height -= new_top - bounds.y;
            bounds.y = new_top;
        }

        // Clip horizontally based on offset direction.
        if offset.x < 0.0 {
            // Item visual is left of its layout position, sliding RIGHT.
            // Reveal the ghost from the left: clip the right edge.
            let available = (visual.x - bounds.x).max(0.0);
            bounds.width = bounds.width.min(available);
        } else if offset.x > 0.0 {
            // Item visual is right of its layout position, sliding LEFT.
            // Reveal the ghost from the right: clip the left edge.
            let new_left = (visual.x + visual.width)
                .max(bounds.x)
                .min(bounds.x + bounds.width);
            bounds.width -= new_left - bounds.x;
            bounds.x = new_left;
        }
    }

    // Ensure non-negative dimensions.
    bounds.width = bounds.width.max(0.0);
    bounds.height = bounds.height.max(0.0);

    bounds
}

/// The default style of a [`TileGrid`].
pub fn default_style(_theme: &Theme) -> Style {
    Style {
        background: None,
        border: Border::default(),
        hovered_region: Highlight {
            background: Background::Color(Color {
                a: 0.08,
                ..Color::BLACK
            }),
            border: Border {
                width: 1.0,
                color: Color {
                    a: 0.15,
                    ..Color::BLACK
                },
                radius: 6.0.into(),
            },
        },
        place_region: None,
        resize_grip: Some(ResizeGrip {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.25,
            },
            dot_size: 2.0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn vec2(x: f32, y: f32) -> Vector {
        Vector::new(x, y)
    }

    #[test]
    fn ghost_unchanged_when_no_animating_items() {
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let result = clip_ghost_for_animating_items(ghost, &[]);
        assert_eq!(result, ghost);
    }

    #[test]
    fn ghost_unchanged_when_items_not_overlapping() {
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        // Item far below ghost, animating but not overlapping.
        let items = [(rect(0.0, 500.0, 280.0, 200.0), vec2(0.0, -50.0))];
        let result = clip_ghost_for_animating_items(ghost, &items);
        assert_eq!(result, ghost);
    }

    #[test]
    fn ghost_grows_from_top_as_item_slides_down() {
        // Ghost at (0, 0, 280, 200).
        // Item displaced from y=0 to y=208 (layout).
        // offset_y starts at -208 (visual at y=0), goes to 0 (visual at y=208).
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 208.0, 280.0, 200.0);

        // At animation start: offset_y = -208, visual at y=0 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -208.0))],
        );
        assert_eq!(result.height, 0.0);
        assert_eq!(result.y, 0.0);

        // At midpoint: offset_y = -104, visual at y=104.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -104.0))],
        );
        assert_eq!(result.height, 104.0);
        assert_eq!(result.y, 0.0);

        // Near end: offset_y = -8, visual at y=200 — no overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -8.0))],
        );
        assert_eq!(result.height, 200.0);
        assert_eq!(result.y, 0.0);
    }

    #[test]
    fn ghost_grows_from_bottom_as_item_slides_up() {
        // Ghost at (0, 208, 280, 200) — from y=208 to y=408.
        // Item displaced from y=208 to y=0 (layout).
        // offset_y starts at 208 (visual at y=208), goes to 0 (visual at y=0).
        let ghost = rect(0.0, 208.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 0.0, 280.0, 200.0);

        // At animation start: offset_y = 208, visual at y=208 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, 208.0))],
        );
        assert_eq!(result.height, 0.0);

        // At midpoint: offset_y = 104, visual at y=104 — bottom of visual is 304.
        // Ghost from 208..408, visual from 104..304. Overlap = 208..304.
        // Ghost should show 304..408, height = 104.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, 104.0))],
        );
        assert!((result.y - 304.0).abs() < f32::EPSILON);
        assert!((result.height - 104.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ghost_grows_from_left_as_item_slides_right() {
        // Ghost at (0, 0, 280, 200).
        // Item displaced from x=0 to x=300 (layout).
        // offset_x starts at -300 (visual at x=0), goes to 0 (visual at x=300).
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(300.0, 0.0, 280.0, 200.0);

        // At animation start: offset_x = -300, visual at x=0 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(-300.0, 0.0))],
        );
        assert_eq!(result.width, 0.0);
        assert_eq!(result.x, 0.0);

        // At midpoint: offset_x = -150, visual at x=150.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(-150.0, 0.0))],
        );
        assert_eq!(result.width, 150.0);
        assert_eq!(result.x, 0.0);
    }

    #[test]
    fn ghost_grows_from_right_as_item_slides_left() {
        // Ghost at (300, 0, 280, 200).
        // Item displaced from x=300 to x=0 (layout).
        // offset_x starts at 300 (visual at x=300), goes to 0 (visual at x=0).
        let ghost = rect(300.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 0.0, 280.0, 200.0);

        // At animation start: offset_x = 300, visual at x=300 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(300.0, 0.0))],
        );
        assert_eq!(result.width, 0.0);

        // At midpoint: offset_x = 150, visual at x=150 — visual right = 430.
        // Ghost from x=300..580. Overlap = 300..430.
        // Ghost should show 430..580, width = 150.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(150.0, 0.0))],
        );
        assert!((result.x - 430.0).abs() < f32::EPSILON);
        assert!((result.width - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ghost_clipped_by_multiple_items() {
        // Ghost at (0, 0, 280, 400).
        // Item A sliding down from top of ghost.
        // Item B sliding up from bottom of ghost.
        let ghost = rect(0.0, 0.0, 280.0, 400.0);

        // Item A: layout at y=200, offset_y = -100, visual at y=100.
        // Clips ghost bottom to y=100.
        let item_a = (rect(0.0, 200.0, 280.0, 200.0), vec2(0.0, -100.0));

        // Item B: layout at y=-200, offset_y = 100, visual at y=-100, bottom=100.
        // Clips ghost top to y=100. But after item A already clipped bottom
        // to height=100 (ghost goes from 0..100), item B visual bottom is 100
        // which equals the ghost bottom, so overlap check: ghost 0..100,
        // visual -100..100 — they overlap. Clip top: new_top = 100.
        // ghost height = 0.
        let item_b = (rect(0.0, -200.0, 280.0, 200.0), vec2(0.0, 100.0));

        let result = clip_ghost_for_animating_items(ghost, &[item_a, item_b]);
        assert_eq!(result.height, 0.0);
    }

    #[test]
    fn ghost_full_size_when_item_animation_complete() {
        // Item has zero offset — animation finished.
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let items = [(rect(0.0, 208.0, 280.0, 200.0), vec2(0.0, 0.0))];
        let result = clip_ghost_for_animating_items(ghost, &items);
        assert_eq!(result, ghost);
    }
}
