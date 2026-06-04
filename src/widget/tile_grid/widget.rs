//! The [`TileGrid`] widget implementation.
//!
//! This module contains the main [`TileGrid`] widget, event types, and the
//! [`Catalog`] trait for theming.
//!
//! `TileGrid` renders a [`State`](super::State) — a recursive tree of grids.
//! A node with children is a *container* (a "group"): its label is drawn in a
//! header strip and the widget lays its child grid out in the body below.
//! Because every node's [`Content`] occupies a disjoint rectangle (a group's
//! header strip never overlaps its children's body cells), the widget can
//! drive layout, drawing and events over one flat, id-sorted list — the
//! recursion lives only in *position* computation and the cross-group drag
//! hit-test. A single drag state machine owns the whole tree, which is what
//! lets a drag gesture move a tile continuously from one group into another.

use std::collections::HashMap;

use iced_widget::container;

use crate::core::keyboard;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay::{self, Group};
use crate::core::renderer;
use crate::core::time::{Duration, Instant};
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    self, Background, Border, Color, Element, Event, Layout, Length, Pixels,
    Point, Rectangle, Shell, Size, Theme, Vector, Widget,
};

use super::content::Content;
use super::engine::{Internal, MoveMode};
use super::item_id::ItemId;
use super::shared::{
    self, DRAG_DEADBAND_DISTANCE, ItemAnimations, RESIZE_CORNER_REACH,
};
use super::state;

/// Upward extension (in pixels) of an item's hover hit-test, so the
/// straddling controls overlay stays visible while the cursor reaches it.
const CONTROLS_HOVER_BAND: f32 = 30.0;

/// How long the drag target cell must stay put before the grid reflows to
/// make room for it. Sweeping a tile across the board re-targets every cell
/// it passes, so without this dwell the whole grid thrashes even though only
/// the final destination matters. The floating tile still tracks the cursor
/// immediately; only the reflow waits.
const DRAG_DWELL: Duration = Duration::from_millis(175);

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
    /// An item move operation within its current grid (drag by title bar).
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
    /// A node moved from one grid into another (cross-group drag).
    ///
    /// The node keeps its [`ItemId`] across the move; it is removed from
    /// its source grid (which compacts to close the gap) and inserted into
    /// the destination grid at `(x, y)`.
    Reparent {
        /// The node being moved. Keeps its id across the transfer.
        node: ItemId,
        /// The destination container, or `None` for the root grid.
        new_parent: Option<ItemId>,
        /// The destination column position.
        x: u16,
        /// The destination row position.
        y: u16,
        /// The phase of the drag interaction.
        phase: DragPhase,
        /// The resolved engine [`MoveMode`] for this drag tick.
        mode: MoveMode,
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
            Self::Reparent { node, .. } => node,
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

    /// Returns `true` if this is any reparent (cross-group) action.
    #[must_use]
    pub fn is_reparent(&self) -> bool {
        matches!(self, Self::Reparent { .. })
    }
}

/// A node collected from the [`State`](super::State) tree while building
/// the widget. Flattened into the parallel `items`/`contents`/… arrays.
struct NodeBuild<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    id: ItemId,
    parent: Option<ItemId>,
    depth: usize,
    held: bool,
    child_engine: Option<&'a Internal>,
    content: Content<'a, Message, Theme, Renderer>,
}

/// Recursively flattens a [`Grid`](super::state::Grid) into `out`, calling
/// `view` once per node and recording its parent, depth and (for
/// containers) child engine.
fn collect_nodes<'a, T, Message, Theme, Renderer>(
    grid: &'a state::Grid<T>,
    parent: Option<ItemId>,
    depth: usize,
    view: &impl Fn(ItemId, &'a T) -> Content<'a, Message, Theme, Renderer>,
    out: &mut Vec<NodeBuild<'a, Message, Theme, Renderer>>,
) where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    for (id, node) in grid.iter() {
        let content = view(id, &node.data);
        let held = content.is_held();
        let child_engine = node.children.as_ref().map(state::Grid::engine);
        out.push(NodeBuild {
            id,
            parent,
            depth,
            held,
            child_engine,
            content,
        });
        if let Some(child) = node.children.as_ref() {
            collect_nodes(child, Some(id), depth + 1, view, out);
        }
    }
}

/// A grid-based layout widget inspired by [GridStack.js](https://gridstackjs.com/).
///
/// Items are placed on a discrete grid with a fixed number of columns. Each item
/// occupies an integer-sized rectangle `(x, y, w, h)` in grid coordinates.
/// Users can drag items by their title bars to move them, or drag the bottom-right
/// edges to resize them. A node with children is rendered as a *group*: a header
/// strip with its label, and a nested grid below.
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
    /// The root grid's engine.
    root_engine: &'a Internal,
    /// All nodes in the tree, sorted by [`ItemId`].
    items: Vec<ItemId>,
    /// Per-node content, aligned with `items`.
    contents: Vec<Content<'a, Message, Theme, Renderer>>,
    /// Per-node parent container (`None` = root), aligned with `items`.
    parents: Vec<Option<ItemId>>,
    /// Per-node child engine (`Some` = container), aligned with `items`.
    child_engines: Vec<Option<&'a Internal>>,
    /// Per-node nesting depth (0 = root level), aligned with `items`.
    depths: Vec<usize>,
    /// Per-node held flag, aligned with `items`.
    held: Vec<bool>,
    /// Maps an [`ItemId`] to its index in the parallel arrays.
    index_of: HashMap<ItemId, usize>,
    width: Length,
    height: Length,
    spacing: f32,
    cell_height: CellHeight,
    /// Pixel height reserved for a labeled group's header strip.
    group_header: f32,
    /// Pixel padding inside a container, between its border and the child
    /// grid.
    group_padding: f32,
    /// When `true`, containers are sized to fit their children (plus the
    /// header and padding) rather than using their authored row span.
    size_to_content: bool,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    locked: bool,
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
    /// The view function is called once for **every** node in the tree (at
    /// any depth), receiving the node's [`ItemId`] and a reference to its
    /// user data. For a container node, the returned [`Content`]'s title bar
    /// is the group's header; the widget renders the child grid in the body.
    ///
    /// Prefer the [`tile_grid`](super::super::tile_grid) helper function.
    ///
    /// [`State`]: super::State
    pub(crate) fn new<T>(
        state: &'a state::State<T>,
        view: impl Fn(ItemId, &'a T) -> Content<'a, Message, Theme, Renderer>,
    ) -> Self {
        let mut builds: Vec<NodeBuild<'a, Message, Theme, Renderer>> =
            Vec::new();
        collect_nodes(state.root(), None, 0, &view, &mut builds);
        builds.sort_by_key(|b| b.id);

        let len = builds.len();
        let mut items = Vec::with_capacity(len);
        let mut contents = Vec::with_capacity(len);
        let mut parents = Vec::with_capacity(len);
        let mut child_engines = Vec::with_capacity(len);
        let mut depths = Vec::with_capacity(len);
        let mut held = Vec::with_capacity(len);
        let mut index_of = HashMap::with_capacity(len);

        for (i, build) in builds.into_iter().enumerate() {
            index_of.insert(build.id, i);
            items.push(build.id);
            contents.push(build.content);
            parents.push(build.parent);
            child_engines.push(build.child_engine);
            depths.push(build.depth);
            held.push(build.held);
        }

        Self {
            root_engine: state.engine(),
            items,
            contents,
            parents,
            child_engines,
            depths,
            held,
            index_of,
            width: Length::Fill,
            height: Length::Shrink,
            spacing: 0.0,
            cell_height: CellHeight::default(),
            group_header: 0.0,
            group_padding: 0.0,
            size_to_content: false,
            on_action: None,
            locked: false,
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

    /// Sets the pixel height reserved for a labeled group's header strip.
    ///
    /// A container node whose [`Content`] has a title bar reserves this
    /// many pixels at the top for the label; its child grid fills the
    /// remaining body. Containers without a title bar reserve nothing.
    /// Defaults to `0.0`, which leaves flat (group-less) grids unchanged.
    pub fn group_header(mut self, height: impl Into<Pixels>) -> Self {
        self.group_header = height.into().0;
        self
    }

    /// Sets the pixel padding inside a container, between its border and the
    /// child grid. Defaults to `0.0`.
    pub fn group_padding(mut self, padding: impl Into<Pixels>) -> Self {
        self.group_padding = padding.into().0;
        self
    }

    /// Sizes containers to fit their children (header + child grid +
    /// padding) instead of their authored row span. Group nodes resize
    /// automatically as tiles are added, removed, or dragged in or out.
    ///
    /// Works best with a [`CellHeight::Fixed`] cell height shared by every
    /// level. Defaults to `false`, leaving flat grids unchanged.
    pub fn size_to_content(mut self, size_to_content: bool) -> Self {
        self.size_to_content = size_to_content;
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

    // ── tree lookups ───────────────────────────────────────────

    fn idx(&self, id: ItemId) -> usize {
        self.index_of[&id]
    }

    fn parent_of(&self, id: ItemId) -> Option<ItemId> {
        self.parents[self.idx(id)]
    }

    fn child_engine_of(&self, id: ItemId) -> Option<&'a Internal> {
        self.child_engines[self.idx(id)]
    }

    fn is_group(&self, id: ItemId) -> bool {
        self.child_engine_of(id).is_some()
    }

    fn depth_of(&self, id: ItemId) -> usize {
        self.depths[self.idx(id)]
    }

    fn has_title_bar(&self, id: ItemId) -> bool {
        self.contents[self.idx(id)].has_title_bar()
    }

    /// Returns the engine governing the grid owned by `owner` (`None` =
    /// root).
    fn owner_engine(&self, owner: Option<ItemId>) -> &Internal {
        match owner {
            None => self.root_engine,
            Some(group) => {
                self.child_engine_of(group).expect("owner is a container")
            }
        }
    }

    /// Returns the size (in cells) of `id` from its grid's engine.
    fn size_of(&self, id: ItemId) -> (u16, u16) {
        self.owner_engine(self.parent_of(id))
            .get(id)
            .map_or((1, 1), |node| (node.w, node.h))
    }

    /// Collects the held ids that live directly in the grid owned by
    /// `owner` (collisions resolve within a single grid).
    fn held_in(&self, owner: Option<ItemId>) -> Vec<ItemId> {
        self.items
            .iter()
            .copied()
            .filter(|&id| {
                self.parent_of(id) == owner && self.held[self.idx(id)]
            })
            .collect()
    }

    /// Returns `true` if `ancestor` is an ancestor of `id` in the tree.
    fn is_ancestor(&self, ancestor: ItemId, id: ItemId) -> bool {
        let mut parent = self.parent_of(id);
        while let Some(p) = parent {
            if p == ancestor {
                return true;
            }
            parent = self.parent_of(p);
        }
        false
    }

    /// Computes `(cell_width, cell_height)` for a grid of `columns` columns
    /// laid out across `width` pixels.
    fn grid_cell_dims(&self, columns: u16, width: f32) -> (f32, f32) {
        let cols = f32::from(columns.max(1));
        let cell_w = (width - (cols - 1.0) * self.spacing) / cols;
        let cell_h = match self.cell_height {
            CellHeight::Auto => cell_w,
            CellHeight::Fixed(h) => h,
        };
        (cell_w, cell_h)
    }

    /// Total root-grid height, for `Length::Shrink` resolution.
    fn root_height(&self, cell_h: f32) -> f32 {
        let rows = f32::from(self.root_engine.get_row());
        if rows == 0.0 {
            0.0
        } else {
            rows * cell_h + (rows - 1.0) * self.spacing
        }
    }

    // ── recursive positioning ──────────────────────────────────

    /// Computes the relative pixel rects for every node, applying any
    /// in-flight drag/resize preview and size-to-content fitting.
    fn positions(
        &self,
        total_width: f32,
        drag: Option<DragMove>,
        resize: Option<ResizeTarget>,
    ) -> Positions {
        let mut out = Positions::default();
        let area = Rectangle {
            x: 0.0,
            y: 0.0,
            width: total_width,
            height: 0.0,
        };
        self.place_grid(None, area, drag, resize, &mut out);
        out
    }

    /// Builds the (possibly preview-mutated) engine for the grid owned by
    /// `owner`, or `None` if this grid is unaffected by the active drag /
    /// resize and the committed engine can be used directly.
    fn preview_engine(
        &self,
        owner: Option<ItemId>,
        drag: Option<DragMove>,
        resize: Option<ResizeTarget>,
    ) -> Option<Internal> {
        if let Some(d) = drag {
            if d.source_owner == owner && d.dest_owner == owner {
                // Within-grid move. The snapshot scopes `try_swap_pack` to
                // this move; clear it afterwards (as the committed path in
                // `State::perform` does) so it cannot leak into the
                // `size_to_content` resize pass and spuriously reorder
                // sibling groups under Swap mode.
                let mut engine = self.owner_engine(owner).clone();
                engine.save_snapshot();
                engine.move_item_held(
                    d.id,
                    d.x,
                    d.y,
                    &self.held_in(owner),
                    d.mode,
                );
                engine.clear_snapshot();
                return Some(engine);
            }
            if d.source_owner == owner {
                // Reparent: remove from the source grid (gravity closes up).
                let mut engine = self.owner_engine(owner).clone();
                engine.remove_item(d.id);
                return Some(engine);
            }
            if d.dest_owner == owner {
                // Reparent: insert into the destination grid.
                let mut engine = self.owner_engine(owner).clone();
                engine.add_item_with_id(d.id, d.x, d.y, d.w, d.h);
                return Some(engine);
            }
        }
        if let Some(r) = resize
            && self.parent_of(r.id) == owner
        {
            let mut engine = self.owner_engine(owner).clone();
            engine.resize_item_held(r.id, r.w, r.h, &self.held_in(owner));
            return Some(engine);
        }
        None
    }

    /// Returns the engine for the grid owned by `owner`, with the active
    /// drag/resize preview applied and — when `size_to_content` is on —
    /// every group child resized to fit its own content.
    fn fitted_engine(
        &self,
        owner: Option<ItemId>,
        cell_h: f32,
        drag: Option<DragMove>,
        resize: Option<ResizeTarget>,
    ) -> Internal {
        let mut engine = self
            .preview_engine(owner, drag, resize)
            .unwrap_or_else(|| self.owner_engine(owner).clone());

        if self.size_to_content {
            let groups: Vec<ItemId> = engine
                .items()
                .filter(|node| self.is_group(node.id))
                .map(|node| node.id)
                .collect();
            for gid in groups {
                let inner = self.fitted_engine(Some(gid), cell_h, drag, resize);
                let used_rows = inner.get_row();
                let used_cols = inner.get_col();

                // Height = children rows + header/padding rows.
                let height =
                    (self.group_extra_rows(gid, cell_h) + used_rows).max(1);

                // Width = the children's used columns, scaled from the
                // group's inner column count to its authored outer span
                // (so unused columns are trimmed).
                let authored_w =
                    engine.get(gid).map_or(1, |node| node.w).max(1);
                let inner_cols = self
                    .child_engine_of(gid)
                    .map_or(1, Internal::columns)
                    .max(1);
                let width = (u32::from(used_cols) * u32::from(authored_w))
                    .div_ceil(u32::from(inner_cols))
                    as u16;

                engine.resize_item(gid, width.max(1), height);
            }
            engine.pack_nodes();
        }
        engine
    }

    /// Rows occupied by a group's header strip plus its vertical padding.
    fn group_extra_rows(&self, gid: ItemId, cell_h: f32) -> u16 {
        let header = if self.has_title_bar(gid) {
            self.group_header
        } else {
            0.0
        };
        let extra_px = header + 2.0 * self.group_padding;
        if cell_h <= 0.0 {
            0
        } else {
            (extra_px / cell_h).ceil() as u16
        }
    }

    fn place_grid(
        &self,
        owner: Option<ItemId>,
        area: Rectangle,
        drag: Option<DragMove>,
        resize: Option<ResizeTarget>,
        out: &mut Positions,
    ) {
        let (cell_w, cell_h) =
            self.grid_cell_dims(self.owner_engine(owner).columns(), area.width);
        let engine = self.fitted_engine(owner, cell_h, drag, resize);

        // For a content-fitted group, `fitted_engine` trimmed the outer width
        // to the used column span. Divide the body by that same span (rather
        // than the authored inner column count) so each cell keeps its
        // authored pixel size — otherwise a child that frees up columns makes
        // the remaining cells shrink (the resize "overshoots") and leaves a
        // gap to the group border.
        let cell_w = if self.size_to_content && owner.is_some() {
            self.grid_cell_dims(engine.get_col(), area.width).0
        } else {
            cell_w
        };
        let regions = shared::compute_regions_for(
            &engine,
            area.width,
            cell_w,
            cell_h,
            self.spacing,
        );

        for (id, (px, py, pw, ph)) in regions {
            let rect = Rectangle {
                x: area.x + px,
                y: area.y + py,
                width: pw,
                height: ph,
            };

            // A node only acts as a container if it is known to this widget
            // (it always is) and has a child engine.
            if self.index_of.contains_key(&id) && self.is_group(id) {
                out.group_rects.insert(id, rect);

                let header = if self.has_title_bar(id) {
                    self.group_header.min(rect.height)
                } else {
                    0.0
                };

                let pad = self.group_padding;

                // The header strip shares the body's horizontal inset so the
                // group title lines up with the left edge of its child tiles
                // rather than sitting flush against the group border.
                out.content.insert(
                    id,
                    Rectangle {
                        x: rect.x + pad,
                        y: rect.y,
                        width: (rect.width - 2.0 * pad).max(0.0),
                        height: header,
                    },
                );

                let body = Rectangle {
                    x: rect.x + pad,
                    y: rect.y + header + pad,
                    width: (rect.width - 2.0 * pad).max(0.0),
                    height: (rect.height - header - 2.0 * pad).max(0.0),
                };
                out.bodies.insert(id, body);
                self.place_grid(Some(id), body, drag, resize, out);
            } else {
                out.content.insert(id, rect);
            }
        }
    }
}

/// The relative pixel rects produced by [`TileGrid::positions`].
#[derive(Default)]
struct Positions {
    /// Per-node Content rect — the header strip for a container, the full
    /// cell for a leaf.
    content: HashMap<ItemId, Rectangle>,
    /// Per-container padded body rect (where its child grid is laid out).
    bodies: HashMap<ItemId, Rectangle>,
    /// Per-container full rect (header + body), used for the drag ghost.
    group_rects: HashMap<ItemId, Rectangle>,
}

/// Ephemeral interaction state stored in the widget tree.
#[derive(Debug, Default, Clone)]
enum Interaction {
    /// No interaction in progress.
    #[default]
    Idle,
    /// The user is dragging a node to move it (within its grid or into
    /// another).
    Moving {
        /// The node being moved.
        id: ItemId,
        /// The cursor position when the drag started.
        origin: Point,
        /// Offset from the node's top-left corner to the grab point.
        grab_offset: Vector,
        /// The node's grid position when the drag started.
        start_x: u16,
        start_y: u16,
        /// Cell dimensions of the node's source grid at drag start.
        cell_w: f32,
        cell_h: f32,
        /// Whether a `DragPhase::Started` event has already been emitted.
        started: bool,
    },
    /// The user is dragging an edge to resize a node.
    Resizing {
        /// The node being resized.
        id: ItemId,
        /// The cursor position when the drag started.
        origin: Point,
        /// The node's grid size when the drag started.
        start_w: u16,
        start_h: u16,
        /// Cell dimensions of the node's grid at drag start.
        cell_w: f32,
        cell_h: f32,
        /// Whether a `DragPhase::Started` event has already been emitted.
        started: bool,
    },
}

/// The tentative target of an active drag, used to build the preview layout.
///
/// When `source_owner == dest_owner` this is a within-grid move; otherwise
/// it is a cross-group reparent.
#[derive(Debug, Clone, Copy)]
struct DragMove {
    id: ItemId,
    source_owner: Option<ItemId>,
    dest_owner: Option<ItemId>,
    x: u16,
    y: u16,
    mode: MoveMode,
    w: u16,
    h: u16,
    /// When this target cell was first seen. The reflow waits [`DRAG_DWELL`]
    /// past this before applying, so sweeping across cells doesn't thrash.
    since: Instant,
}

impl DragMove {
    /// Whether two targets land the dragged node in the same cell of the same
    /// grid (ignoring the dwell timestamp). A move that keeps the same cell
    /// must not reset the dwell — otherwise a jittery hand parked on the
    /// destination never settles.
    fn same_cell(&self, other: &Self) -> bool {
        self.source_owner == other.source_owner
            && self.dest_owner == other.dest_owner
            && self.x == other.x
            && self.y == other.y
    }

    /// Whether the target has dwelled long enough for the grid to reflow.
    fn dwelled(&self) -> bool {
        self.since.elapsed() >= DRAG_DWELL
    }
}

/// The resize target during an active resize.
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
    /// Tentative drag target during an active drag — the cell the cursor is
    /// currently over. Tracked every frame to detect the dwell; the preview
    /// is driven by [`held_drag`](Self::held_drag), not this.
    drag_target: Option<DragMove>,
    /// The last drag target that *dwelled* long enough to apply. This drives
    /// the preview layout and persists between dwells, so the reflow it
    /// produced stays put while the tile keeps moving — the next dwell adjusts
    /// from it rather than snapping back to the committed layout.
    held_drag: Option<DragMove>,
    /// Tentative resize dimensions during an active resize.
    resize_target: Option<ResizeTarget>,
    /// Body rects of container nodes from the last layout pass, in
    /// widget-relative coordinates. Used to hit-test the reparent target.
    group_bodies: HashMap<ItemId, Rectangle>,
    /// Container body rects captured at the start of the active drag, before
    /// the preview moves anything. The reparent hit-test resolves against
    /// these fixed pre-lift rects rather than the live (preview) bodies — so
    /// the drop target is a stable function of the cursor, not of the
    /// evolving layout (which would feed back and oscillate).
    drag_bodies: Option<HashMap<ItemId, Rectangle>>,
    /// Full rects of container nodes from the last layout pass, in
    /// widget-relative coordinates. Used for the whole-group drag ghost.
    group_rects: HashMap<ItemId, Rectangle>,
    /// The item whose controls overlay should be shown (the hovered item,
    /// with an upward band so reaching the straddling controls keeps them
    /// visible). `None` hides all controls.
    controls_hovered: Option<ItemId>,
    /// Most recently seen keyboard modifiers.
    modifiers: keyboard::Modifiers,
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

        // ItemId is monotonically increasing and the flat list is sorted by
        // id, so new nodes always appear at the end. Remove states for nodes
        // that no longer exist, then diff_children_custom reconciles.
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
        #[allow(unreachable_patterns)]
        let resolved_width = match self.width {
            Length::Fill | Length::FillPortion(_) => max.width,
            Length::Shrink => max.width,
            Length::Fixed(w) => w.min(max.width),
            _ => max.width,
        };

        let (_, root_cell_h) =
            self.grid_cell_dims(self.root_engine.columns(), resolved_width);
        let grid_h = self.root_height(root_cell_h);

        #[allow(unreachable_patterns)]
        let resolved_height = match self.height {
            Length::Fill | Length::FillPortion(_) => max.height,
            Length::Shrink => grid_h.min(max.height),
            Length::Fixed(h) => h.min(max.height),
            _ => max.height,
        };

        let bounds = Size::new(resolved_width, resolved_height);

        let memory = tree.state.downcast_mut::<Memory>();
        // Promote the current target to the held target once it has dwelled
        // (and lands in a new cell). The held target persists between dwells,
        // so the reflow it produced stays put while the tile keeps moving —
        // the grid doesn't snap back to committed on every sideways nudge.
        if let Some(t) = memory.drag_target
            && t.dwelled()
            && memory.held_drag.is_none_or(|h| !h.same_cell(&t))
        {
            memory.held_drag = Some(t);
        }
        let drag = memory.held_drag;
        let resize = memory.resize_target;

        // Compute relative node rects + container bodies, applying the
        // in-flight drag/resize preview and size-to-content fitting.
        let positions = self.positions(resolved_width, drag, resize);

        let regions: Vec<(ItemId, (f32, f32, f32, f32))> = positions
            .content
            .iter()
            .map(|(&id, r)| (id, (r.x, r.y, r.width, r.height)))
            .collect();

        let dragged_id = match &memory.interaction {
            Interaction::Moving { id, .. }
            | Interaction::Resizing { id, .. } => Some(*id),
            Interaction::Idle => None,
        };
        let now = memory.animations.now.unwrap_or_else(Instant::now);
        memory
            .animations
            .update_positions(&regions, dragged_id, now);

        // The ghost tracks the dragged node's snap rect — the *full* rect
        // for a container, so dragging a group previews the whole group. Only
        // while the move has dwelled (so `drag` is applied): before that the
        // slot hasn't opened, and we want the highlight to reveal in place
        // rather than slide in from the tile's origin.
        if drag.is_some()
            && let Some(dragged) = dragged_id
            && let Some(snap) = positions
                .group_rects
                .get(&dragged)
                .or_else(|| positions.content.get(&dragged))
        {
            memory.animations.update_ghost_position(*snap, now);
        }

        memory.group_bodies = positions.bodies;
        memory.group_rects = positions.group_rects;

        let content_rects = positions.content;
        let children = self
            .items
            .iter()
            .zip(&mut self.contents)
            .zip(tree.children.iter_mut())
            .map(|((id, content), tree)| {
                if let Some(rect) = content_rects.get(id) {
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

        // Propagate events to contents.
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
            held_drag,
            drag_bodies,
            resize_target,
            group_bodies,
            modifiers,
            ..
        } = tree.state.downcast_mut();

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                *modifiers = *m;
                if let Some(target) = drag_target {
                    target.mode = self.swap_mode.resolve(m.shift());
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::Unfocused) => {
                *modifiers = keyboard::Modifiers::empty();
                if let Some(target) = drag_target {
                    target.mode = self.swap_mode.resolve(false);
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                animations.now = Some(*now);

                // While a drag is live, force a *relayout* every frame — not
                // just a repaint. The dwell is time-based and evaluated in
                // `layout`; `request_redraw` alone reuses the cached layout,
                // so with the cursor sitting still the reflow would never
                // compute. Invalidating layout re-runs the dwell check, and
                // the steady frame loop then plays the reflow animation.
                if !matches!(interaction, Interaction::Idle) {
                    shell.invalidate_layout();
                    shell.request_redraw();
                } else if animations.is_animating(*now) {
                    shell.request_redraw();
                }

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

                    // Resize first (bottom-right edge of a leaf).
                    if self.on_action.is_some()
                        && !self.locked
                        && let Some(resize_id) =
                            self.find_resize_target(layout, cursor_position)
                        && let Some((w, h)) = self
                            .owner_engine(self.parent_of(resize_id))
                            .get(resize_id)
                            .map(|n| (n.w, n.h))
                    {
                        let (cell_w, cell_h) = self.grid_cell_dims(
                            self.owner_engine(self.parent_of(resize_id))
                                .columns(),
                            self.owner_width(
                                self.parent_of(resize_id),
                                group_bodies,
                                bounds,
                            ),
                        );
                        *interaction = Interaction::Resizing {
                            id: resize_id,
                            origin: cursor_position,
                            start_w: w,
                            start_h: h,
                            cell_w,
                            cell_h,
                            started: false,
                        };
                        return;
                    }

                    self.click_item(
                        interaction,
                        layout,
                        cursor_position,
                        shell,
                        group_bodies,
                        bounds,
                    );
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                match interaction {
                    Interaction::Moving { id, started, .. } if *started => {
                        // Commit what the preview showed: the held (dwelled)
                        // slot, or — if the tile was dropped before any dwell —
                        // the cell the cursor is currently over.
                        if let Some(on_action) = &self.on_action
                            && let Some(target) = held_drag.or(*drag_target)
                        {
                            let action =
                                if target.dest_owner == target.source_owner {
                                    Action::Move {
                                        id: *id,
                                        x: target.x,
                                        y: target.y,
                                        phase: DragPhase::Ended,
                                        mode: target.mode,
                                    }
                                } else {
                                    Action::Reparent {
                                        node: *id,
                                        new_parent: target.dest_owner,
                                        x: target.x,
                                        y: target.y,
                                        phase: DragPhase::Ended,
                                        mode: target.mode,
                                    }
                                };
                            shell.publish(on_action(action));
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
                *held_drag = None;
                *drag_bodies = None;
                *resize_target = None;
                animations.hide_ghost();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => {
                match interaction {
                    Interaction::Moving {
                        id,
                        origin,
                        grab_offset,
                        start_x,
                        start_y,
                        cell_w,
                        cell_h,
                        started,
                    } => {
                        if let Some(on_action) = &self.on_action
                            && let Some(cursor_position) = cursor.position()
                            && cursor_position.distance(*origin)
                                > DRAG_DEADBAND_DISTANCE
                        {
                            let now =
                                animations.now.unwrap_or_else(Instant::now);
                            animations.show_ghost(*id, now);

                            let mode =
                                self.swap_mode.resolve(modifiers.shift());
                            let source_owner = self.parent_of(*id);
                            // Resolve the reparent target against the fixed
                            // pre-lift bodies, captured once at drag start, so
                            // the drop is a stable function of the cursor and
                            // doesn't chase the preview as groups reflow.
                            let bodies: &HashMap<ItemId, Rectangle> =
                                drag_bodies.get_or_insert_with(|| {
                                    group_bodies.clone()
                                });
                            let dest_owner = self.reparent_target(
                                bodies,
                                origin_offset(bounds),
                                cursor_position,
                                *id,
                            );

                            let stamp = Instant::now();
                            let (cx, cy) = if dest_owner == source_owner {
                                // Within-grid move: snap by delta.
                                let step_w = *cell_w + self.spacing;
                                let step_h = *cell_h + self.spacing;
                                let grid_dx = ((cursor_position.x - origin.x)
                                    / step_w)
                                    .round()
                                    as i32;
                                let grid_dy = ((cursor_position.y - origin.y)
                                    / step_h)
                                    .round()
                                    as i32;
                                (
                                    (*start_x as i32 + grid_dx).max(0) as u16,
                                    (*start_y as i32 + grid_dy).max(0) as u16,
                                )
                            } else {
                                // Cross-group: land at the cursor's cell in
                                // the destination grid.
                                let node_top_left = Point::new(
                                    cursor_position.x - grab_offset.x,
                                    cursor_position.y - grab_offset.y,
                                );
                                self.dest_cell(
                                    dest_owner,
                                    bodies,
                                    bounds.position(),
                                    bounds,
                                    node_top_left,
                                )
                            };
                            let (w, h) = self.size_of(*id);
                            let candidate = DragMove {
                                id: *id,
                                source_owner,
                                dest_owner,
                                x: cx,
                                y: cy,
                                mode,
                                w,
                                h,
                                since: stamp,
                            };

                            // Debounce the reflow: only re-target (and re-arm
                            // the dwell redraw / re-publish) when the cell
                            // actually changes. The floating tile tracks the
                            // cursor every frame regardless; a parked cursor —
                            // jitter and all — keeps its `since` and settles.
                            if drag_target
                                .is_none_or(|prev| !prev.same_cell(&candidate))
                            {
                                *drag_target = Some(candidate);
                                shell.request_redraw_at(stamp + DRAG_DWELL);
                                let phase = phase_of(started);
                                let action = if dest_owner == source_owner {
                                    Action::Move {
                                        id: *id,
                                        x: cx,
                                        y: cy,
                                        phase,
                                        mode,
                                    }
                                } else {
                                    Action::Reparent {
                                        node: *id,
                                        new_parent: dest_owner,
                                        x: cx,
                                        y: cy,
                                        phase,
                                        mode,
                                    }
                                };
                                shell.publish(on_action(action));
                            }
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

                            *resize_target = Some(ResizeTarget {
                                id: *id,
                                w: new_w,
                                h: new_h,
                            });

                            shell.publish(on_action(Action::Resize {
                                id: *id,
                                w: new_w,
                                h: new_h,
                                phase: phase_of(started),
                            }));
                        }
                        shell.request_redraw();
                    }
                    Interaction::Idle => {}
                }
            }
            _ => {}
        }

        // Track which node the cursor hovers so we request redraws when it
        // changes (keeps title-bar controls + overlay controls in sync).
        //
        // Only recompute while the cursor position is actually known. When
        // our own controls overlay sits under the cursor, iced reports its
        // (non-`None`) `mouse_interaction` and hands the base widget
        // `Cursor::Unavailable` (see `UserInterface::update`). Clearing the
        // hover then would drop the overlay, which un-captures the cursor on
        // the next frame and re-shows it — a per-frame flicker that also
        // makes clicks land on the wrong phase. So hold the last hover while
        // the cursor is unavailable, and only force-clear it when the cursor
        // genuinely leaves the window.
        {
            let hover =
                if matches!(event, Event::Mouse(mouse::Event::CursorLeft)) {
                    Some((None, None))
                } else {
                    cursor.position().map(|point| {
                        let over = bounds.contains(point).then_some(point);

                        let hovered_index = over.and_then(|pos| {
                            layout
                                .children()
                                .enumerate()
                                .find(|(_, child)| child.bounds().contains(pos))
                                .map(|(i, _)| i)
                        });

                        // The hovered item for the controls overlay. The overlay
                        // straddles the item's top edge, so the cursor leaves the
                        // item rect to reach it — extend the hit-test upward by a
                        // band so the controls stay visible while being aimed at.
                        let controls_hovered = over.and_then(|pos| {
                            self.items
                                .iter()
                                .copied()
                                .zip(layout.children())
                                .find(|(_, child)| child.bounds().contains(pos))
                                .or_else(|| {
                                    self.items
                                        .iter()
                                        .copied()
                                        .zip(layout.children())
                                        .find(|(_, child)| {
                                            let b = child.bounds();
                                            Rectangle {
                                                x: b.x,
                                                y: b.y - CONTROLS_HOVER_BAND,
                                                width: b.width,
                                                height: b.height
                                                    + CONTROLS_HOVER_BAND,
                                            }
                                            .contains(pos)
                                        })
                                })
                                .map(|(id, _)| id)
                        });

                        (hovered_index, controls_hovered)
                    })
                };

            if let Some((hovered_index, controls_hovered)) = hover {
                let memory = tree.state.downcast_mut::<Memory>();
                let mut redraw = false;
                if memory.last_hovered != hovered_index {
                    memory.last_hovered = hovered_index;
                    redraw = true;
                }
                if memory.controls_hovered != controls_hovered {
                    memory.controls_hovered = controls_hovered;
                    redraw = true;
                }
                if redraw {
                    shell.request_redraw();
                }
            }
        }

        // Detect mouse interaction (cursor type) changes for hover effects.
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

        if self.on_action.is_some()
            && !self.locked
            && let Some(cursor_position) = cursor.position_over(bounds)
            && self.find_resize_target(layout, cursor_position).is_some()
        {
            return mouse::Interaction::ResizingDiagonallyDown;
        }

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
            held_drag,
            group_rects,
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

        // Draw nodes in parent-before-child order so a container's chrome
        // sits behind the tiles it holds.
        for (id, content, tree, item_layout) in self.draw_order(tree, layout) {
            match picked_item {
                Some((dragging, grab_offset)) if id == dragging => {
                    render_picked =
                        Some(((content, tree), item_layout, grab_offset));
                }
                _ => {
                    let draw_cursor = if resizing_id == Some(id) {
                        let b = item_layout.bounds();
                        mouse::Cursor::Available(Point::new(
                            b.x + b.width / 2.0,
                            b.y + b.height / 2.0,
                        ))
                    } else {
                        item_cursor
                    };

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

        let grid_style = Catalog::style(theme, &self.class);

        // Persistent border framing every container's full rect.
        if let Some(border) = grid_style.group_border {
            let offset = layout.bounds().position();
            for rect in group_rects.values() {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: rect.x + offset.x,
                            y: rect.y + offset.y,
                            ..*rect
                        },
                        border,
                        ..renderer::Quad::default()
                    },
                    Background::Color(Color::TRANSPARENT),
                );
            }
        }

        // Resize grip indicator on hovered, resizable leaves.

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
                if self.is_group(id) || !content.is_resizable() {
                    continue;
                }
                if picked_item.is_some_and(|(pid, _)| pid == id) {
                    continue;
                }

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

        // Render the picked node last, floating under the cursor.
        if let Some(((content, tree), item_layout, grab_offset)) = render_picked
            && let Some(cursor_position) = cursor.position()
        {
            let dragged_id = picked_item.unwrap().0;

            // The drop ghost is the dragged node's full rect — the whole
            // group (header + body) when dragging a container, not just its
            // header strip.
            let widget_origin = layout.bounds().position();
            let snap_bounds = group_rects.get(&dragged_id).map_or_else(
                || item_layout.bounds(),
                |r| Rectangle {
                    x: r.x + widget_origin.x,
                    y: r.y + widget_origin.y,
                    ..*r
                },
            );
            let ghost_pos_offset = animations.ghost_offset(now);
            let ghost_target = Rectangle {
                x: snap_bounds.x + ghost_pos_offset.x,
                y: snap_bounds.y + ghost_pos_offset.y,
                ..snap_bounds
            };
            let ghost_alpha = animations.ghost_alpha(now);
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

            let ghost_bounds = shared::clip_ghost_for_animating_items(
                ghost_target,
                &animating,
            );

            // The drop-slot highlight tracks the held (dwelled) target, which
            // is also what the grid has reflowed to open. While the tile is
            // still sweeping between dwells, the highlight stays at the held
            // slot rather than chasing the cursor.
            if held_drag.is_some() {
                let highlight = match held_drag.map(|t| t.mode) {
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
                            color: highlight
                                .border
                                .color
                                .scale_alpha(ghost_alpha),
                            ..highlight.border
                        },
                        ..renderer::Quad::default()
                    },
                    highlight.background.scale_alpha(ghost_alpha),
                );
            }

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
        // Controls overlays render only for the hovered item, and never while
        // a drag or resize is in flight — otherwise they float around with the
        // reflowing layout and get in the way of the gesture.
        let memory = tree.state.downcast_ref::<Memory>();
        let controls_hovered = memory.controls_hovered;
        let interacting = !matches!(memory.interaction, Interaction::Idle);

        let children = self
            .items
            .iter()
            .copied()
            .zip(&mut self.contents)
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|(((id, content), state), layout)| {
                let show_controls =
                    !interacting && controls_hovered == Some(id);
                content.overlay(
                    state,
                    layout,
                    renderer,
                    viewport,
                    translation,
                    show_controls,
                )
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
    /// Yields `(id, content, tree, layout)` in parent-before-child order
    /// (DFS pre-order from the root), so container chrome draws beneath the
    /// tiles it holds.
    #[allow(clippy::type_complexity)]
    fn draw_order<'b>(
        &'b self,
        tree: &'b Tree,
        layout: Layout<'b>,
    ) -> Vec<(
        ItemId,
        &'b Content<'a, Message, Theme, Renderer>,
        &'b Tree,
        Layout<'b>,
    )> {
        let layouts: Vec<Layout<'b>> = layout.children().collect();
        let mut out = Vec::with_capacity(self.items.len());
        self.push_draw_order(None, tree, &layouts, &mut out);
        out
    }

    #[allow(clippy::type_complexity)]
    fn push_draw_order<'b>(
        &'b self,
        parent: Option<ItemId>,
        tree: &'b Tree,
        layouts: &[Layout<'b>],
        out: &mut Vec<(
            ItemId,
            &'b Content<'a, Message, Theme, Renderer>,
            &'b Tree,
            Layout<'b>,
        )>,
    ) {
        for (i, &id) in self.items.iter().enumerate() {
            if self.parents[i] != parent {
                continue;
            }
            out.push((id, &self.contents[i], &tree.children[i], layouts[i]));
            if self.is_group(id) {
                self.push_draw_order(Some(id), tree, layouts, out);
            }
        }
    }

    /// The pixel width of the grid owned by `owner` (root or a container's
    /// body), from the last layout's body rects.
    fn owner_width(
        &self,
        owner: Option<ItemId>,
        bodies: &HashMap<ItemId, Rectangle>,
        bounds: Rectangle,
    ) -> f32 {
        match owner {
            None => bounds.width,
            Some(group) => bodies.get(&group).map_or(bounds.width, |b| b.width),
        }
    }

    /// The deepest valid container whose body contains the cursor, or
    /// `None` for the root grid. Excludes the dragged node and its own
    /// subtree (which would create a cycle).
    fn reparent_target(
        &self,
        bodies: &HashMap<ItemId, Rectangle>,
        widget_origin: Vector,
        cursor: Point,
        dragged: ItemId,
    ) -> Option<ItemId> {
        let mut best: Option<(usize, ItemId)> = None;
        for &id in &self.items {
            if !self.is_group(id) || id == dragged {
                continue;
            }
            if self.is_ancestor(dragged, id) {
                continue;
            }
            if let Some(body) = bodies.get(&id) {
                let abs = Rectangle {
                    x: body.x + widget_origin.x,
                    y: body.y + widget_origin.y,
                    ..*body
                };
                if abs.contains(cursor) {
                    let depth = self.depth_of(id);
                    if best.is_none_or(|(bd, _)| depth > bd) {
                        best = Some((depth, id));
                    }
                }
            }
        }
        best.map(|(_, id)| id)
    }

    /// Converts a floating node's top-left pixel to a cell in the
    /// destination grid (root or a container body).
    fn dest_cell(
        &self,
        dest_owner: Option<ItemId>,
        bodies: &HashMap<ItemId, Rectangle>,
        widget_origin: Point,
        bounds: Rectangle,
        node_top_left: Point,
    ) -> (u16, u16) {
        let (body_x, body_y, width) = match dest_owner {
            None => (widget_origin.x, widget_origin.y, bounds.width),
            Some(group) => match bodies.get(&group) {
                Some(b) => {
                    (b.x + widget_origin.x, b.y + widget_origin.y, b.width)
                }
                None => (widget_origin.x, widget_origin.y, bounds.width),
            },
        };
        let (cell_w, cell_h) =
            self.grid_cell_dims(self.owner_engine(dest_owner).columns(), width);
        let step_w = cell_w + self.spacing;
        let step_h = cell_h + self.spacing;
        let x = ((node_top_left.x - body_x) / step_w).round().max(0.0) as u16;
        let y = ((node_top_left.y - body_y) / step_h).round().max(0.0) as u16;
        (x, y)
    }

    /// Finds a resizable leaf whose bottom-right corner is near the cursor.
    fn find_resize_target(
        &self,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> Option<ItemId> {
        for ((id, content), item_layout) in self
            .items
            .iter()
            .copied()
            .zip(&self.contents)
            .zip(layout.children())
        {
            if self.is_group(id) || !content.is_resizable() {
                continue;
            }
            let rect = item_layout.bounds();
            let dx = (rect.x + rect.width) - cursor_position.x;
            let dy = (rect.y + rect.height) - cursor_position.y;
            if dx >= 0.0 && dy >= 0.0 && dx + dy < RESIZE_CORNER_REACH {
                return Some(id);
            }
        }
        None
    }

    /// Computes the cursor interaction from the drag state, without
    /// consulting child widgets.
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
            && self.find_resize_target(layout, cursor_position).is_some()
        {
            return Some(mouse::Interaction::ResizingDiagonallyDown);
        }

        None
    }

    /// Handles a click on a node, potentially starting a drag.
    fn click_item(
        &self,
        interaction: &mut Interaction,
        layout: Layout<'_>,
        cursor_position: Point,
        shell: &mut Shell<'_, Message>,
        bodies: &HashMap<ItemId, Rectangle>,
        bounds: Rectangle,
    ) {
        let clicked = self
            .items
            .iter()
            .copied()
            .zip(&self.contents)
            .zip(layout.children())
            .find(|((_, _), layout)| layout.bounds().contains(cursor_position));

        if let Some(((id, content), item_layout)) = clicked {
            let in_drag_zone = !self.locked
                && content.is_draggable()
                && content.can_be_dragged_at(item_layout, cursor_position);

            if !in_drag_zone && let Some(on_action) = &self.on_action {
                shell.publish(on_action(Action::Click(id)));
            }

            if self.on_action.is_some()
                && in_drag_zone
                && let Some(item) =
                    self.owner_engine(self.parent_of(id)).get(id)
            {
                let item_pos = item_layout.bounds().position();
                let grab_offset = Vector::new(
                    cursor_position.x - item_pos.x,
                    cursor_position.y - item_pos.y,
                );

                let owner = self.parent_of(id);
                let (cell_w, cell_h) = self.grid_cell_dims(
                    self.owner_engine(owner).columns(),
                    self.owner_width(owner, bodies, bounds),
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

/// The drag phase implied by whether a `Started` event has been emitted.
fn phase_of(started: &mut bool) -> DragPhase {
    if *started {
        DragPhase::Ongoing
    } else {
        *started = true;
        DragPhase::Started
    }
}

/// The widget's absolute origin as a [`Vector`].
fn origin_offset(bounds: Rectangle) -> Vector {
    Vector::new(bounds.x, bounds.y)
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
    /// A persistent border drawn around every container's full rect (header
    /// + body). `None` disables it. Useful to frame groups while editing.
    pub group_border: Option<Border>,
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
        group_border: None,
    }
}
