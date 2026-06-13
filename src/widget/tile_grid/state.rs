//! User-facing state for the tile grid.
//!
//! [`State`] is a **recursive tree** of grid nodes. Each [`Node`] pairs user
//! data of type `T` with an optional child [`Grid`] — a node with children is
//! a *container* (a "group"); a node without is a leaf tile. A flat grid is
//! simply the depth-1 case where no node has children.
//!
//! Every [`Grid`] owns its own [`Internal`] layout engine, reused verbatim at
//! every depth. Item ids are minted from a **single global allocator** on
//! [`State`], so an [`ItemId`] is unique across the whole tree and a node keeps
//! its id when it moves between grids.

use std::collections::BTreeMap;

use super::ItemId;
use super::configuration::{Configuration, Item};
use super::engine::{self, Internal, fit_group_width};
use super::rect::Rect;
use super::widget::{Action, DragPhase};

/// A node in the recursive grid tree.
///
/// Pairs user data with an optional child [`Grid`]. When `children` is
/// `Some`, this node is a *container* (a "group") whose body hosts a nested
/// grid; when `None`, it is a leaf tile.
#[derive(Debug, Clone)]
pub struct Node<T> {
    /// Application-specific data associated with this node.
    pub data: T,
    /// The child grid, present when this node is a container.
    pub children: Option<Grid<T>>,
}

impl<T> Node<T> {
    /// Returns `true` if this node is a container (has a child grid).
    #[must_use]
    pub fn is_group(&self) -> bool {
        self.children.is_some()
    }

    /// Returns `true` if `id` is somewhere within this node's subtree.
    fn subtree_contains(&self, id: ItemId) -> bool {
        self.children
            .as_ref()
            .is_some_and(|grid| grid.find_node(id).is_some())
    }
}

/// How a group's width is determined within its parent grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// The group's width tracks its content, and its children may grow up to
    /// the parent grid's full column count. The default for new groups.
    Shrink,
    /// The group's width is pinned to `n` columns and its children are capped
    /// at it, regardless of content.
    Fixed(u16),
}

impl From<u16> for Width {
    /// A bare column count is a [`Fixed`](Width::Fixed)-width group, so
    /// `add_group(rect, 4, data)` reads as "4 columns wide".
    fn from(columns: u16) -> Self {
        Width::Fixed(columns)
    }
}

/// One level of the recursive layout: a layout [`Internal`] engine paired
/// with the user data for the nodes it arranges.
///
/// Node ids are **not** minted here — they come from the owning [`State`]'s
/// global allocator (see [`State`]). Iteration is by [`ItemId`] order
/// (`BTreeMap`), which the widget's tree reconciliation relies on.
#[derive(Debug, Clone)]
pub struct Grid<T> {
    /// The layout engine for this level.
    engine: Internal,
    /// This level's nodes, keyed by id.
    nodes: BTreeMap<ItemId, Node<T>>,
    /// How this grid's *group* is sized in its parent (irrelevant for the
    /// root). Drives [`fit_widths`](Self::fit_widths) and the inner column
    /// count.
    width: Width,
}

impl<T> Grid<T> {
    /// Creates an empty `Shrink` grid with the given number of columns.
    fn new(columns: u16) -> Self {
        Self::with_mode(columns, Width::Shrink)
    }

    /// Creates an empty grid with the given column count and width mode.
    fn with_mode(columns: u16, width_mode: Width) -> Self {
        Self {
            engine: Internal::new(columns),
            nodes: BTreeMap::new(),
            width: width_mode,
        }
    }

    /// Returns the layout engine for this level.
    #[must_use]
    pub fn engine(&self) -> &Internal {
        &self.engine
    }

    /// Returns the number of columns at this level.
    #[must_use]
    pub fn columns(&self) -> u16 {
        self.engine.columns()
    }

    /// Returns an iterator over the `(ItemId, &Node)` pairs at this level,
    /// in id order.
    pub fn iter(&self) -> impl Iterator<Item = (ItemId, &Node<T>)> {
        self.nodes.iter().map(|(&id, node)| (id, node))
    }

    /// Returns the number of nodes directly at this level.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Walks this subtree, pushing every leaf (a node with no children) as
    /// `(id, parent, &data)` onto `out`. `parent` is the id of the group
    /// node directly containing the leaf, or `None` at the root. Group nodes
    /// are descended but never emitted — they carry no leaf payload.
    fn collect_leaves<'a>(
        &'a self,
        parent: Option<ItemId>,
        out: &mut Vec<(ItemId, Option<ItemId>, &'a T)>,
    ) {
        for (&id, node) in &self.nodes {
            match node.children.as_ref() {
                Some(child) => child.collect_leaves(Some(id), out),
                None => out.push((id, parent, &node.data)),
            }
        }
    }

    /// Mutable [`collect_leaves`](Self::collect_leaves).
    fn collect_leaves_mut<'a>(
        &'a mut self,
        parent: Option<ItemId>,
        out: &mut Vec<(ItemId, Option<ItemId>, &'a mut T)>,
    ) {
        for (&id, node) in &mut self.nodes {
            match node.children.as_mut() {
                Some(child) => child.collect_leaves_mut(Some(id), out),
                None => out.push((id, parent, &mut node.data)),
            }
        }
    }

    /// Returns `true` if this level has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the grid that directly contains `id`, searching this level
    /// and all descendants.
    fn grid_containing(&self, id: ItemId) -> Option<&Grid<T>> {
        if self.nodes.contains_key(&id) {
            return Some(self);
        }
        self.nodes
            .values()
            .filter_map(|node| node.children.as_ref())
            .find_map(|child| child.grid_containing(id))
    }

    /// Mutable [`grid_containing`](Self::grid_containing).
    fn grid_containing_mut(&mut self, id: ItemId) -> Option<&mut Grid<T>> {
        if self.nodes.contains_key(&id) {
            return Some(self);
        }
        self.nodes
            .values_mut()
            .filter_map(|node| node.children.as_mut())
            .find_map(|child| child.grid_containing_mut(id))
    }

    /// Finds a node anywhere in this subtree.
    fn find_node(&self, id: ItemId) -> Option<&Node<T>> {
        if let Some(node) = self.nodes.get(&id) {
            return Some(node);
        }
        self.nodes
            .values()
            .filter_map(|node| node.children.as_ref())
            .find_map(|child| child.find_node(id))
    }

    /// Mutable [`find_node`](Self::find_node).
    fn find_node_mut(&mut self, id: ItemId) -> Option<&mut Node<T>> {
        if self.nodes.contains_key(&id) {
            return self.nodes.get_mut(&id);
        }
        self.nodes
            .values_mut()
            .filter_map(|node| node.children.as_mut())
            .find_map(|child| child.find_node_mut(id))
    }

    /// Collects the ids of held nodes at this level (collisions are
    /// resolved within a single grid, so only this level matters).
    fn held_ids(&self, is_held: &impl Fn(ItemId, &T) -> bool) -> Vec<ItemId> {
        self.nodes
            .iter()
            .filter(|&(&id, node)| is_held(id, &node.data))
            .map(|(&id, _)| id)
            .collect()
    }

    /// Shrinks every container node's width to the column extent its children
    /// actually occupy — size-to-content for *width*.
    ///
    /// This is the one place a group's width is decided: the committed engine
    /// holds the truth and the widget draws it. (Width is pure column math, so
    /// it belongs in the state; height is left to the widget because it
    /// depends on the cell pixel size.) Without this, a group kept its
    /// authored width and over-claimed columns — positioning and the
    /// right-edge clamp treated a 6-wide group as 8 wide, so it could never
    /// sit flush against the board's right edge.
    ///
    /// Only `Shrink` groups are fitted; a `Fixed(n)` group keeps its pinned
    /// width. Depth-first: a nested group is fitted before its parent measures
    /// it, so the parent sees its child's *fitted* extent. An empty group
    /// keeps its current width (it has no content to measure).
    fn fit_widths(&mut self) {
        for node in self.nodes.values_mut() {
            if let Some(child) = node.children.as_mut() {
                child.fit_widths();
            }
        }

        let cols = self.engine.columns();
        let fits: Vec<(ItemId, u16, u16)> = self
            .nodes
            .iter()
            .filter_map(|(&id, node)| {
                let child = node.children.as_ref()?;
                if !matches!(child.width, Width::Shrink) {
                    return None;
                }
                let used = child.engine.get_col();
                if used == 0 {
                    return None;
                }
                let h = self.engine.get(id)?.h;
                Some((id, fit_group_width(used, cols), h))
            })
            .collect();

        for (id, w, h) in fits {
            self.engine.resize_item(id, w, h);
        }
    }
}

/// User-facing state: a recursive tree of [`Grid`]s with a single global
/// [`ItemId`] allocator.
///
/// This is the tile-grid analogue of iced's [`pane_grid::State<T>`], but
/// where `pane_grid` is a binary split tree, this is an `(x, y, w, h)`
/// coordinate grid that nests to arbitrary depth.
///
/// # Example
///
/// ```
/// use sweeten::widget::tile_grid::State;
///
/// let mut state: State<String> = State::new(12);
///
/// let id = state.add([0, 0, 4, 2], "Widget A".to_string());
/// assert_eq!(state.get(id), Some(&"Widget A".to_string()));
///
/// state.remove(id);
/// ```
///
/// [`pane_grid::State<T>`]: https://docs.iced.rs/iced/widget/pane_grid/state/struct.State.html
#[derive(Debug, Clone)]
pub struct State<T> {
    /// The root grid.
    root: Grid<T>,
    /// A sibling grid the widget never lays out, draws, or drags — it is
    /// invisible to every widget pass, which only ever reads [`root`](Self::root).
    /// Its leaves still participate in the data-facing surface
    /// ([`iter_leaves`](Self::iter_leaves), [`get`](Self::get)) so a caller can
    /// render them through a separate (non-dragged) region while keeping ONE
    /// `State`, ONE dispatch path, and ONE persistence snapshot. The id
    /// allocator is shared, so rail ids never collide with root ids.
    rail: Grid<T>,
    /// Monotonic global id allocator shared across every nested grid.
    next_id: usize,
    /// When set, every committing mutation re-fits each group's width to its
    /// content (see [`Grid::fit_widths`]). Mirror of the widget's
    /// `size_to_content`: with it on, the committed group footprint matches
    /// what the widget draws, so groups position and clamp at their true
    /// width rather than their authored one.
    fit: bool,
}

impl<T> State<T> {
    /// Creates a new empty grid state with the given number of columns.
    ///
    /// # Panics
    ///
    /// Panics if `columns` is 0.
    #[must_use]
    pub fn new(columns: u16) -> Self {
        Self {
            root: Grid::new(columns),
            rail: Grid::new(columns),
            next_id: 0,
            fit: false,
        }
    }

    /// Creates a new [`State`] from the given [`Configuration`].
    ///
    /// Items are assigned monotonic [`ItemId`]s in depth-first declaration
    /// order. Collisions are resolved during construction; gravity
    /// compaction runs once per grid at the end (unless `float` is enabled).
    ///
    /// # Example
    ///
    /// ```
    /// use sweeten::widget::tile_grid::{State, Configuration};
    ///
    /// let state: State<&str> = State::with_configuration(
    ///     Configuration::new(12)
    ///         .with_item([0, 0, 4, 2], "left")
    ///         .with_item([4, 0, 8, 2], "right"),
    /// );
    ///
    /// assert_eq!(state.len(), 2);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `config.columns` is 0.
    #[must_use]
    pub fn with_configuration(config: impl Into<Configuration<T>>) -> Self {
        let config = config.into();
        let mut state = Self::new(config.columns);
        state.root.engine.set_max_rows(config.max_rows);

        let mut next_id = 0;
        build_into(&mut state.root, config.items, &mut next_id, config.float);
        state.next_id = next_id;
        state
    }

    /// Creates a new [`State`] from two configurations: a `main` arrangement
    /// (the widget-rendered tree) and a `rail` arrangement (a sibling grid the
    /// widget never touches, see [`rail`](Self::rail)).
    ///
    /// Ids are minted from a single global allocator threaded across both
    /// configurations, so a rail leaf is globally unique against every main
    /// leaf — [`get`](Self::get) / [`iter_leaves`](Self::iter_leaves) reach
    /// either side by id with no collision. The `rail` columns are taken from
    /// its own configuration; only `main` governs the root engine.
    ///
    /// # Panics
    ///
    /// Panics if either `config.columns` is 0.
    #[must_use]
    pub fn with_configurations(
        main: impl Into<Configuration<T>>,
        rail: impl Into<Configuration<T>>,
    ) -> Self {
        let main = main.into();
        let rail = rail.into();
        let mut state = Self::new(main.columns);
        state.rail = Grid::new(rail.columns);
        state.root.engine.set_max_rows(main.max_rows);
        state.rail.engine.set_max_rows(rail.max_rows);

        let mut next_id = 0;
        build_into(&mut state.root, main.items, &mut next_id, main.float);
        build_into(&mut state.rail, rail.items, &mut next_id, rail.float);
        state.next_id = next_id;
        state
    }

    /// Allocates the next global [`ItemId`].
    fn alloc_id(&mut self) -> ItemId {
        let id = ItemId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Returns the root grid.
    #[must_use]
    pub fn root(&self) -> &Grid<T> {
        &self.root
    }

    /// Returns the sibling rail grid — the leaves the widget never lays out
    /// or draws, intended for rendering through a separate (non-dragged)
    /// region. See the [`rail`](Self::rail) field.
    #[must_use]
    pub fn rail(&self) -> &Grid<T> {
        &self.rail
    }

    /// Returns the number of columns in the root grid.
    #[must_use]
    pub fn columns(&self) -> u16 {
        self.root.engine.columns()
    }

    /// Returns the root layout engine.
    ///
    /// Use [`engine_of`](Self::engine_of) for a nested node's engine.
    #[must_use]
    pub fn engine(&self) -> &Internal {
        &self.root.engine
    }

    /// Returns the engine of the grid that directly contains `id`.
    #[must_use]
    pub fn engine_of(&self, id: ItemId) -> Option<&Internal> {
        self.root.grid_containing(id).map(Grid::engine)
    }

    /// Sets float mode on the root engine.
    pub fn set_float(&mut self, float: bool) {
        self.root.engine.set_float(float);
    }

    /// Returns whether float mode is enabled on the root engine.
    #[must_use]
    pub fn float(&self) -> bool {
        self.root.engine.float()
    }

    /// Sizes every group to its content rather than its authored span.
    ///
    /// This is the single switch for the whole tree: the widget reads it off
    /// the [`State`] it is given (there is no separate widget-side flag). With
    /// it on, each group's **width** is committed to the column extent its
    /// children occupy after every mutation, so a group positions and clamps
    /// at the width it is actually drawn — a content-narrowed group can sit
    /// flush against the grid's right edge instead of being held back by its
    /// authored width. (Group **height** is still resolved by the widget at
    /// layout time, since it depends on the cell pixel height.)
    pub fn fit(&mut self, enabled: bool) {
        self.fit = enabled;
        self.fit_groups();
    }

    /// Returns whether groups are sized to their content. See
    /// [`fit`](Self::fit).
    #[must_use]
    pub fn fits(&self) -> bool {
        self.fit
    }

    /// Re-fits every group's width to its content, if [`fit`](Self::fit) is on.
    fn fit_groups(&mut self) {
        if self.fit {
            self.root.fit_widths();
        }
    }

    /// Adds a new leaf item at the root with associated data.
    ///
    /// Returns the [`ItemId`] of the newly created item.
    pub fn add(&mut self, rect: impl Into<Rect>, user_data: T) -> ItemId {
        let rect = rect.into();
        let id = self.alloc_id();
        self.root
            .engine
            .add_item_with_id(id, rect.x, rect.y, rect.w, rect.h);
        self.root.nodes.insert(
            id,
            Node {
                data: user_data,
                children: None,
            },
        );
        id
    }

    /// Adds a new leaf item at the root with auto-placement.
    ///
    /// Returns `None` if the grid is full (only possible when `max_rows`
    /// is set).
    pub fn add_auto(&mut self, w: u16, h: u16, user_data: T) -> Option<ItemId> {
        let (x, y) = self.root.engine.find_empty_position(w, h)?;
        Some(self.add([x, y, w, h], user_data))
    }

    /// Adds a new *container* (group) node at the root.
    ///
    /// `width` is the group's sizing mode: [`Width::Shrink`] (the default —
    /// the group tracks its content and its children may grow up to the root's
    /// full column count; see [`fit`](Self::fit)) or [`Width::Fixed(n)`] (the
    /// group is pinned to `n` columns and its children capped at it). A bare
    /// `u16` means `Fixed`, so `add_group([8, 0, 4, 1], 4, data)` and
    /// `add_group([8, 0, 4, 1], Width::Fixed(4), data)` are the same.
    ///
    /// For a `Shrink` group `rect`'s width is the initial span; for a `Fixed`
    /// group the mode's column count governs and `rect`'s width is ignored.
    /// Use [`add_child`](Self::add_child) to populate it.
    ///
    /// [`Width::Fixed(n)`]: Width::Fixed
    pub fn add_group(
        &mut self,
        rect: impl Into<Rect>,
        width: impl Into<Width>,
        user_data: T,
    ) -> ItemId {
        let rect = rect.into();
        let mode = width.into();
        // A Shrink group spans the root's full column count (so children can
        // grow into it) and starts at its authored width; a Fixed(n) group is
        // exactly `n` columns.
        let (w, inner_cols) = match mode {
            Width::Shrink => (rect.w, self.root.engine.columns()),
            Width::Fixed(n) => (n, n),
        };
        let id = self.alloc_id();
        self.root
            .engine
            .add_item_with_id(id, rect.x, rect.y, w, rect.h);
        self.root.nodes.insert(
            id,
            Node {
                data: user_data,
                children: Some(Grid::with_mode(inner_cols, mode)),
            },
        );
        id
    }

    /// Adds a leaf item inside the container node `parent`.
    ///
    /// Returns `None` if `parent` does not exist or is not a container.
    pub fn add_child(
        &mut self,
        parent: ItemId,
        rect: impl Into<Rect>,
        user_data: T,
    ) -> Option<ItemId> {
        let rect = rect.into();
        let id = self.alloc_id();
        let grid = self.root.find_node_mut(parent)?.children.as_mut()?;
        grid.engine
            .add_item_with_id(id, rect.x, rect.y, rect.w, rect.h);
        grid.nodes.insert(
            id,
            Node {
                data: user_data,
                children: None,
            },
        );
        self.fit_groups();
        Some(id)
    }

    /// Removes a node (anywhere in the tree) and returns its data.
    ///
    /// Removing a container also drops its entire subtree.
    /// Returns `None` if no node with the given ID exists.
    pub fn remove(&mut self, id: ItemId) -> Option<T> {
        let grid = self.root.grid_containing_mut(id)?;
        grid.engine.remove_item(id)?;
        let data = grid.nodes.remove(&id).map(|node| node.data);
        self.fit_groups();
        data
    }

    /// Moves a node to a new position within its current grid.
    ///
    /// Returns `true` if the node was actually moved.
    pub fn move_item(&mut self, id: ItemId, x: u16, y: u16) -> bool {
        let moved = self
            .root
            .grid_containing_mut(id)
            .is_some_and(|grid| grid.engine.move_item(id, x, y));
        if moved {
            self.fit_groups();
        }
        moved
    }

    /// Resizes a node within its current grid.
    ///
    /// Returns `true` if the node was actually resized.
    pub fn resize_item(&mut self, id: ItemId, w: u16, h: u16) -> bool {
        let resized = self
            .root
            .grid_containing_mut(id)
            .is_some_and(|grid| grid.engine.resize_item(id, w, h));
        if resized {
            self.fit_groups();
        }
        resized
    }

    /// Returns a reference to the user data for the given node.
    ///
    /// Searches the root tree first, then the sibling [`rail`](Self::rail), so
    /// a rail leaf is reachable by id exactly like a root leaf.
    #[must_use]
    pub fn get(&self, id: ItemId) -> Option<&T> {
        self.root
            .find_node(id)
            .or_else(|| self.rail.find_node(id))
            .map(|node| &node.data)
    }

    /// Returns a mutable reference to the user data for the given node.
    ///
    /// Searches the root tree first, then the sibling [`rail`](Self::rail).
    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut T> {
        if self.root.find_node(id).is_some() {
            return self.root.find_node_mut(id).map(|node| &mut node.data);
        }
        self.rail.find_node_mut(id).map(|node| &mut node.data)
    }

    /// Returns the full [`Node`] for the given id, anywhere in the tree.
    #[must_use]
    pub fn get_node(&self, id: ItemId) -> Option<&Node<T>> {
        self.root.find_node(id)
    }

    /// Returns the grid item (position/size) for the given ID.
    ///
    /// Searches the root tree first, then the sibling [`rail`](Self::rail), so
    /// a rail leaf's geometry is reachable for a layout snapshot.
    #[must_use]
    pub fn get_item(&self, id: ItemId) -> Option<&engine::Node> {
        self.root
            .grid_containing(id)
            .or_else(|| self.rail.grid_containing(id))
            .and_then(|grid| grid.engine.get(id))
    }

    /// Returns an iterator over the root-level `(ItemId, &T)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (ItemId, &T)> {
        self.root.nodes.iter().map(|(&id, node)| (id, &node.data))
    }

    /// Returns an iterator over the root-level `(ItemId, &mut T)` pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ItemId, &mut T)> {
        self.root
            .nodes
            .iter_mut()
            .map(|(&id, node)| (id, &mut node.data))
    }

    /// Returns an iterator over every **leaf** in the tree, paired with the
    /// id of the group that directly contains it (`None` at the root).
    ///
    /// Unlike [`iter`](Self::iter), which yields only root-level nodes, this
    /// descends into groups and yields each leaf tile no matter how deep it
    /// is nested. Group (container) nodes are walked but never yielded — they
    /// hold no leaf payload of their own. Leaves are returned in id order
    /// within each grid, parents before their descendants.
    /// Leaves from both the root tree and the sibling [`rail`](Self::rail) are
    /// yielded (root first, then rail), so data dispatch over a `State` reaches
    /// rail tiles for free even though the widget never renders them.
    pub fn iter_leaves(
        &self,
    ) -> impl Iterator<Item = (ItemId, Option<ItemId>, &T)> {
        let mut out = Vec::new();
        self.root.collect_leaves(None, &mut out);
        self.rail.collect_leaves(None, &mut out);
        out.into_iter()
    }

    /// Mutable [`iter_leaves`](Self::iter_leaves).
    pub fn iter_leaves_mut(
        &mut self,
    ) -> impl Iterator<Item = (ItemId, Option<ItemId>, &mut T)> {
        let mut out = Vec::new();
        self.root.collect_leaves_mut(None, &mut out);
        self.rail.collect_leaves_mut(None, &mut out);
        out.into_iter()
    }

    /// Returns an iterator over every leaf in the sibling [`rail`](Self::rail)
    /// grid only, paired with the id of the group that directly contains it
    /// (`None` at the rail root). The rail-only counterpart of
    /// [`iter_leaves`](Self::iter_leaves) — the briefing region renders these
    /// without touching the widget-drawn root tree.
    pub fn rail_leaves(
        &self,
    ) -> impl Iterator<Item = (ItemId, Option<ItemId>, &T)> {
        let mut out = Vec::new();
        self.rail.collect_leaves(None, &mut out);
        out.into_iter()
    }

    /// Returns the number of root-level nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.root.nodes.len()
    }

    /// Returns `true` if the root grid has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root.nodes.is_empty()
    }

    /// Returns the current root grid height (max bottom edge of any node).
    #[must_use]
    pub fn get_row(&self) -> u16 {
        self.root.engine.get_row()
    }

    /// Converts the root items to pixel rectangles.
    ///
    /// See [`Internal::item_regions`] for details.
    #[must_use]
    pub fn item_regions(
        &self,
        bounds: (f32, f32),
        spacing: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        self.root.engine.item_regions(bounds, spacing)
    }

    /// Performs an [`Action`] on the grid state.
    ///
    /// The `is_held` predicate determines which items are immovable
    /// (e.g. pinned items that should not be displaced by collisions).
    ///
    /// Click actions are informational -- they do not mutate state. The
    /// caller should inspect the action *before* calling `perform` if it
    /// needs to react to clicks (e.g. to update a focus tracker).
    ///
    /// Move and resize actions are deferred during `Started`/`Ongoing`
    /// phases — the widget computes a visual preview by cloning the engine.
    /// Only `Ended` commits the final layout to the engine.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if action.is_click() {
    ///     self.focus = Some(action.id());
    /// }
    /// self.state.perform(action, |_, item| item.is_pinned);
    /// ```
    pub fn perform(
        &mut self,
        action: Action,
        is_held: impl Fn(ItemId, &T) -> bool,
    ) {
        match action {
            Action::Click(_) => {
                // Click is informational -- no state mutation needed.
            }
            Action::Move {
                id,
                x,
                y,
                phase,
                mode,
                ..
            } => {
                match phase {
                    DragPhase::Started | DragPhase::Ongoing => {
                        // No engine mutation. The widget computes a
                        // visual preview internally by cloning the
                        // engine, so the committed state stays at
                        // the pre-drag layout until the user drops.
                    }
                    DragPhase::Ended => {
                        if let Some(grid) = self.root.grid_containing_mut(id) {
                            // The same call the widget's preview makes
                            // (`preview_engine`), so the committed move equals
                            // the previewed one — the drop lands where shown.
                            let held = grid.held_ids(&is_held);
                            grid.engine.save_snapshot();
                            grid.engine.move_item_held(id, x, y, &held, mode);
                            grid.engine.clear_snapshot();
                        }
                    }
                }
            }
            Action::Resize {
                id, w, h, phase, ..
            } => {
                match phase {
                    DragPhase::Started | DragPhase::Ongoing => {
                        // No engine mutation. See `Move` above.
                    }
                    DragPhase::Ended => {
                        if let Some(grid) = self.root.grid_containing_mut(id) {
                            let held = grid.held_ids(&is_held);
                            grid.engine.resize_item_held(id, w, h, &held);
                        }
                    }
                }
            }
            Action::Reparent {
                node,
                new_parent,
                x,
                y,
                phase,
                ..
            } => {
                match phase {
                    DragPhase::Started | DragPhase::Ongoing => {
                        // Deferred, like `Move`/`Resize`: the widget
                        // previews the transfer by cloning both the
                        // source and destination engines. Nothing is
                        // committed until the user drops.
                    }
                    DragPhase::Ended => {
                        self.apply_reparent(node, new_parent, x, y);
                    }
                }
            }
        }

        // A committed move/resize/reparent can change a group's content
        // extent (a child resized, a tile left or joined a group), so re-fit
        // every group's width to match what `size_to_content` draws.
        self.fit_groups();
    }

    /// Commits a cross-grid transfer: removes `node` (and its subtree) from
    /// its current grid and inserts it into the destination grid (the root
    /// when `new_parent` is `None`) at `(x, y)`, preserving its id and size.
    ///
    /// The move is validated first and is a no-op if the destination does
    /// not exist, is not a container, or lies within `node`'s own subtree
    /// (which would create a cycle). The node is never dropped.
    fn apply_reparent(
        &mut self,
        node: ItemId,
        new_parent: Option<ItemId>,
        x: u16,
        y: u16,
    ) {
        // Validate the destination before mutating anything.
        let dest_valid = match new_parent {
            None => true,
            Some(parent) => {
                parent != node
                    && self.root.find_node(parent).is_some_and(Node::is_group)
                    && !self
                        .root
                        .find_node(node)
                        .is_some_and(|n| n.subtree_contains(parent))
            }
        };
        if !dest_valid {
            return;
        }

        // Read the node's *desired* width (not its grid-clamped width) and
        // remove it (with its subtree) from its source grid, which compacts
        // to close the gap. Carrying the desired width lets the node re-expand
        // when it lands in a grid with room, instead of staying stuck at a
        // width a narrower source grid had clamped it to.
        let Some((w, h, removed)) =
            self.root.grid_containing_mut(node).and_then(|src| {
                let (w, h) =
                    src.engine.get(node).map(|n| (n.desired_w, n.h))?;
                src.engine.remove_item(node);
                src.nodes.remove(&node).map(|data| (w, h, data))
            })
        else {
            return;
        };

        // Insert into the destination grid (validated above to exist).
        let dest = match new_parent {
            None => &mut self.root,
            Some(parent) => self
                .root
                .find_node_mut(parent)
                .and_then(|n| n.children.as_mut())
                .expect("destination validated before removal"),
        };
        // The same call the widget's preview makes (`preview_engine`), so the
        // committed reparent matches the previewed one.
        dest.engine.add_item_with_id(node, x, y, w, h);
        dest.nodes.insert(node, removed);
    }
}

/// Recursively populates `grid` from configuration `items`, threading the
/// global id allocator `next_id`.
///
/// Uses batch mode so gravity compaction runs once per grid rather than
/// after every insertion.
fn build_into<T>(
    grid: &mut Grid<T>,
    items: Vec<Item<T>>,
    next_id: &mut usize,
    float: bool,
) {
    if float {
        grid.engine.set_float(true);
    }
    grid.engine.begin_batch();

    for item in items {
        let id = ItemId(*next_id);
        *next_id += 1;

        grid.engine
            .add_item_with_id(id, item.x, item.y, item.w, item.h);

        let has_constraints = item.min_w.is_some()
            || item.max_w.is_some()
            || item.min_h.is_some()
            || item.max_h.is_some();
        if has_constraints {
            grid.engine.set_item_constraints(
                id, item.min_w, item.max_w, item.min_h, item.max_h,
            );
        }

        let children = if item.is_group() {
            // Groups default to Shrink: the inner grid spans the parent's full
            // column count so children can grow into it, and the group's width
            // tracks its content.
            let mut child = Grid::new(grid.engine.columns());
            build_into(&mut child, item.children, next_id, float);
            Some(child)
        } else {
            None
        };

        grid.nodes.insert(
            id,
            Node {
                data: item.state,
                children,
            },
        );
    }

    grid.engine.end_batch();
}

#[cfg(test)]
mod tests {
    use super::super::configuration::Item;
    use super::super::engine::MoveMode;
    use super::*;

    #[test]
    fn perform_move_does_not_modify_engine_during_drag() {
        // The engine state should remain unchanged during a drag
        // (Started/Ongoing). Only on DragPhase::Ended should the
        // engine be mutated. The widget handles visual preview
        // internally by cloning the engine.
        let mut state: State<()> = State::new(12);
        let a = state.add([0, 0, 4, 2], ());
        let _b = state.add([4, 0, 4, 2], ());
        let _c = state.add([8, 0, 4, 2], ());

        // Snapshot initial positions.
        let initial: Vec<_> =
            state.engine().items().map(|i| (i.id, i.x, i.y)).collect();

        // Simulate drag: Started, then two Ongoing ticks.
        state.perform(
            Action::Move {
                id: a,
                x: 4,
                y: 0,
                phase: DragPhase::Started,
                mode: MoveMode::Swap,
            },
            |_, _| false,
        );

        let after_started: Vec<_> =
            state.engine().items().map(|i| (i.id, i.x, i.y)).collect();
        assert_eq!(
            initial, after_started,
            "engine must not change on DragPhase::Started"
        );

        state.perform(
            Action::Move {
                id: a,
                x: 4,
                y: 1,
                phase: DragPhase::Ongoing,
                mode: MoveMode::Swap,
            },
            |_, _| false,
        );

        let after_ongoing: Vec<_> =
            state.engine().items().map(|i| (i.id, i.x, i.y)).collect();
        assert_eq!(
            initial, after_ongoing,
            "engine must not change on DragPhase::Ongoing"
        );
    }

    #[test]
    fn perform_resize_does_not_modify_engine_during_drag() {
        // Engine state should remain unchanged during a resize
        // (Started/Ongoing). Only on DragPhase::Ended should the
        // engine be mutated.
        let mut state: State<()> = State::new(12);
        let a = state.add([0, 0, 4, 2], ());
        let _b = state.add([4, 0, 4, 2], ());

        // Snapshot initial state (positions + sizes).
        let initial: Vec<_> = state
            .engine()
            .items()
            .map(|i| (i.id, i.x, i.y, i.w, i.h))
            .collect();

        // Started — no mutation.
        state.perform(
            Action::Resize {
                id: a,
                w: 8,
                h: 2,
                phase: DragPhase::Started,
            },
            |_, _| false,
        );
        let after_started: Vec<_> = state
            .engine()
            .items()
            .map(|i| (i.id, i.x, i.y, i.w, i.h))
            .collect();
        assert_eq!(
            initial, after_started,
            "engine must not change on resize DragPhase::Started"
        );

        // Ongoing — no mutation.
        state.perform(
            Action::Resize {
                id: a,
                w: 10,
                h: 3,
                phase: DragPhase::Ongoing,
            },
            |_, _| false,
        );
        let after_ongoing: Vec<_> = state
            .engine()
            .items()
            .map(|i| (i.id, i.x, i.y, i.w, i.h))
            .collect();
        assert_eq!(
            initial, after_ongoing,
            "engine must not change on resize DragPhase::Ongoing"
        );

        // Ended — commits the resize.
        state.perform(
            Action::Resize {
                id: a,
                w: 8,
                h: 2,
                phase: DragPhase::Ended,
            },
            |_, _| false,
        );
        let item_a = state.get_item(a).expect("item a should exist");
        assert_eq!(
            (item_a.w, item_a.h),
            (8, 2),
            "resize should be committed on DragPhase::Ended"
        );
    }

    #[test]
    fn perform_move_ended_applies_with_swap() {
        // On DragPhase::Ended, the engine should apply the move
        // including swap logic (same-size items swap positions).
        let mut state: State<()> = State::new(12);
        let a = state.add([0, 0, 4, 2], ());
        let b = state.add([4, 0, 4, 2], ());
        state.add([8, 0, 4, 2], ());

        // Drop a at (4,0) — onto b. Same size → swap.
        state.perform(
            Action::Move {
                id: a,
                x: 4,
                y: 0,
                phase: DragPhase::Ended,
                mode: MoveMode::Swap,
            },
            |_, _| false,
        );

        let item_a = state.get_item(a).unwrap();
        let item_b = state.get_item(b).unwrap();

        assert_eq!(
            (item_a.x, item_a.y),
            (4, 0),
            "a should be at the drop position"
        );
        assert_eq!(
            (item_b.x, item_b.y),
            (0, 0),
            "b should swap to a's original position"
        );
    }

    // ── with_configuration tests ────────────────────────────────

    #[test]
    fn with_configuration_empty() {
        let state: State<()> = State::with_configuration(
            Configuration::new(12).max_rows(5).float(true),
        );
        assert!(state.is_empty());
        assert_eq!(state.columns(), 12);
        assert_eq!(state.engine().max_rows(), Some(5));
        assert!(state.float());
    }

    #[test]
    fn with_configuration_monotonic_ids() {
        let state: State<&str> = State::with_configuration(
            Configuration::new(12)
                .with_item([0, 0, 4, 2], "a")
                .with_item([4, 0, 4, 2], "b")
                .with_item([8, 0, 4, 2], "c"),
        );

        let ids: Vec<_> = state.iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 3);
        // IDs are monotonically increasing in declaration order.
        assert!(ids[0] < ids[1] && ids[1] < ids[2]);
    }

    #[test]
    fn with_configuration_preserves_positions() {
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .with_item([0, 0, 4, 2], ())
                .with_item([4, 0, 4, 2], ())
                .with_item([8, 0, 4, 2], ()),
        );

        let mut positions: Vec<_> = state
            .engine()
            .items()
            .map(|n| (n.x, n.y, n.w, n.h))
            .collect();
        positions.sort();
        assert_eq!(positions, vec![(0, 0, 4, 2), (4, 0, 4, 2), (8, 0, 4, 2)],);
    }

    #[test]
    fn with_configuration_resolves_collisions() {
        // Two items at the same position — engine should displace one.
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .with_item([0, 0, 4, 2], ())
                .with_item([0, 0, 4, 2], ()),
        );

        let positions: Vec<_> =
            state.engine().items().map(|n| (n.x, n.y)).collect();

        assert_ne!(
            positions[0], positions[1],
            "overlapping items must be resolved"
        );
    }

    #[test]
    fn with_configuration_gravity_compacts() {
        // Items floating at y=10 should be compacted to y=0 when
        // float is off (default).
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .with_item([0, 10, 4, 2], ())
                .with_item([4, 10, 4, 2], ()),
        );

        for node in state.engine().items() {
            assert_eq!(node.y, 0, "gravity should compact items to y=0");
        }
    }

    #[test]
    fn with_configuration_float_preserves_y() {
        // With float enabled, items should stay at their declared y.
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .float(true)
                .with_item([0, 10, 4, 2], ())
                .with_item([4, 10, 4, 2], ()),
        );

        for node in state.engine().items() {
            assert_eq!(node.y, 10, "float mode should preserve declared y");
        }
    }

    #[test]
    fn with_configuration_constraints_applied() {
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .push(Item::new([0, 0, 2, 1], ()).min_w(4).min_h(3)),
        );

        let node = state.engine().items().next().unwrap();
        assert!(node.w >= 4, "min_w constraint should be applied");
        assert!(node.h >= 3, "min_h constraint should be applied");
    }

    #[test]
    fn with_configuration_max_rows_honored() {
        // An item extending past max_rows should be clamped.
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .max_rows(5)
                .float(true)
                .with_item([0, 0, 4, 10], ()),
        );

        let node = state.engine().items().next().unwrap();
        assert!(node.y + node.h <= 5, "item should not extend past max_rows");
    }

    #[test]
    fn with_configuration_user_data_accessible() {
        let state: State<&str> = State::with_configuration(
            Configuration::new(12)
                .with_item([0, 0, 6, 2], "hello")
                .with_item([6, 0, 6, 2], "world"),
        );

        let mut values: Vec<_> = state.iter().map(|(_, data)| *data).collect();
        values.sort();
        assert_eq!(values, vec!["hello", "world"]);
    }

    // ── recursive tree tests ────────────────────────────────────

    #[test]
    fn global_ids_unique_across_nested_grids() {
        // Ids must be unique across the whole tree, not per-engine.
        let mut state: State<&str> = State::new(12);
        let group = state.add_group([0, 0, 6, 4], Width::Shrink, "group");
        let a = state.add_child(group, [0, 0, 3, 2], "a").unwrap();
        let b = state.add_child(group, [3, 0, 3, 2], "b").unwrap();
        let leaf = state.add([6, 0, 6, 4], "leaf");

        let ids = [group, a, b, leaf];
        for (i, &x) in ids.iter().enumerate() {
            for &y in &ids[i + 1..] {
                assert_ne!(x, y, "ids must be globally unique");
            }
        }
    }

    #[test]
    fn nested_lookup_finds_node_and_item() {
        let mut state: State<&str> = State::new(12);
        let group = state.add_group([0, 0, 6, 4], Width::Shrink, "group");
        let child = state.add_child(group, [1, 1, 2, 2], "child").unwrap();

        // get / get_node reach the nested node.
        assert_eq!(state.get(child), Some(&"child"));
        assert!(state.get_node(child).is_some());
        assert!(!state.get_node(child).unwrap().is_group());
        assert!(state.get_node(group).unwrap().is_group());

        // get_item returns the child's geometry from the child grid
        // (which gravity-compacted it to the top-left).
        let item = state.get_item(child).expect("child item exists");
        assert_eq!((item.w, item.h), (2, 2));
    }

    #[test]
    fn nested_move_targets_child_grid() {
        let mut state: State<()> = State::new(12);
        let group = state.add_group([0, 0, 12, 6], Width::Shrink, ());
        let a = state.add_child(group, [0, 0, 2, 2], ()).unwrap();
        let b = state.add_child(group, [2, 0, 2, 2], ()).unwrap();

        // Move `a` within the child grid onto `b` (same size → swap).
        state.perform(
            Action::Move {
                id: a,
                x: 2,
                y: 0,
                phase: DragPhase::Ended,
                mode: MoveMode::Swap,
            },
            |_, _| false,
        );

        let ia = state.get_item(a).unwrap();
        let ib = state.get_item(b).unwrap();
        assert_eq!((ia.x, ia.y), (2, 0), "a moved to drop position");
        assert_eq!((ib.x, ib.y), (0, 0), "b swapped into a's old slot");
    }

    #[test]
    fn size_to_content_commits_group_width_to_its_content() {
        // A 12-col root with one group authored 8 wide whose two 4-wide tiles
        // fill it. Shrinking a tile leaves the group's content at 6 columns;
        // with `size_to_content` the committed group width must follow, so the
        // group can be dragged flush to the right edge instead of being held
        // back by its authored 8-wide footprint.
        let mut state: State<()> = State::new(12);
        state.fit(true);

        let group = state.add_group([0, 0, 8, 1], Width::Shrink, ());
        let _l = state.add_child(group, [0, 0, 4, 3], ()).unwrap();
        let r = state.add_child(group, [4, 0, 4, 3], ()).unwrap();

        // Content fills all 8 columns → group width is 8.
        assert_eq!(state.get_item(group).unwrap().w, 8);

        // Halve the right tile: content now spans 6 columns.
        assert!(state.resize_item(r, 2, 3));
        assert_eq!(
            state.get_item(group).unwrap().w,
            6,
            "group width follows its content after a child shrinks"
        );

        // The 6-wide group can now reach the right edge (x = 12 - 6); with the
        // stale 8-wide footprint the move clamped to x = 4.
        state.perform(
            Action::Move {
                id: group,
                x: 6,
                y: 0,
                phase: DragPhase::Ended,
                mode: MoveMode::Place,
            },
            |_, _| false,
        );
        assert_eq!(
            state.get_item(group).unwrap().x,
            6,
            "group sits flush right at its content width"
        );
    }

    #[test]
    fn size_to_content_off_keeps_authored_group_width() {
        // Without `size_to_content`, a group keeps the width it was authored
        // with even when its content is narrower (fixed-size container mode).
        let mut state: State<()> = State::new(12);
        let group = state.add_group([0, 0, 8, 1], Width::Shrink, ());
        let _l = state.add_child(group, [0, 0, 4, 3], ()).unwrap();
        let r = state.add_child(group, [4, 0, 4, 3], ()).unwrap();

        assert!(state.resize_item(r, 2, 3));
        assert_eq!(
            state.get_item(group).unwrap().w,
            8,
            "authored width is preserved when size_to_content is off"
        );
    }

    #[test]
    fn iter_leaves_descends_groups_with_parent() {
        // root: a leaf, a group {child1, child2}, and a nested group.
        let mut state: State<&str> = State::new(12);
        let root_leaf = state.add([0, 0, 2, 2], "root_leaf");
        let group = state.add_group([2, 0, 6, 4], Width::Shrink, "group");
        let c1 = state.add_child(group, [0, 0, 2, 2], "c1").unwrap();
        let c2 = state.add_child(group, [2, 0, 2, 2], "c2").unwrap();

        let leaves: Vec<_> = state.iter_leaves().collect();
        // Three leaves; the group node itself is not a leaf.
        assert_eq!(leaves.len(), 3);

        let root = leaves.iter().find(|(id, ..)| *id == root_leaf).unwrap();
        assert_eq!(root.1, None, "root leaf has no parent");
        assert_eq!(*root.2, "root_leaf");

        for (id, parent, data) in &leaves {
            if *id == c1 || *id == c2 {
                assert_eq!(
                    *parent,
                    Some(group),
                    "grouped child sees its group"
                );
            }
            assert_ne!(*id, group, "group node is not yielded as a leaf");
            let _ = data;
        }
    }

    #[test]
    fn iter_leaves_mut_reaches_nested_data() {
        let mut state: State<String> = State::new(12);
        let group = state.add_group([0, 0, 6, 4], Width::Shrink, "g".into());
        state
            .add_child(group, [0, 0, 2, 2], "child".into())
            .unwrap();

        for (_, _, data) in state.iter_leaves_mut() {
            data.push('!');
        }

        let touched: Vec<_> =
            state.iter_leaves().map(|(_, _, d)| d.clone()).collect();
        assert_eq!(touched, vec!["child!".to_string()]);
    }

    #[test]
    fn rail_leaf_reached_by_iter_leaves_and_get_but_not_in_root() {
        // A sibling rail grid is built alongside the main one. Its leaf is
        // reachable through the data-facing surface (`iter_leaves`, `get`,
        // `get_item`) with a globally-unique id, yet it is NOT part of the
        // widget-rendered `root` tree.
        let state: State<&str> = State::with_configurations(
            Configuration::new(12)
                .with_item([0, 0, 4, 2], "main_a")
                .with_item([4, 0, 4, 2], "main_b"),
            Configuration::new(4).with_item([0, 0, 4, 2], "rail_x"),
        );

        // The rail leaf is yielded by iter_leaves alongside the root leaves.
        let leaves: Vec<_> = state.iter_leaves().map(|(_, _, d)| *d).collect();
        assert!(leaves.contains(&"main_a"));
        assert!(leaves.contains(&"main_b"));
        assert!(leaves.contains(&"rail_x"));

        // Find the rail leaf's id and confirm it is reachable by `get` and
        // `get_item`, with geometry from the rail grid.
        let rail_id = state
            .iter_leaves()
            .find(|(_, _, d)| **d == "rail_x")
            .map(|(id, ..)| id)
            .expect("rail leaf is enumerated");
        assert_eq!(state.get(rail_id), Some(&"rail_x"));
        assert!(state.get_item(rail_id).is_some(), "rail leaf has geometry");

        // The id is globally unique against the root leaves.
        let root_ids: Vec<_> = state.root().iter().map(|(id, _)| id).collect();
        assert!(
            !root_ids.contains(&rail_id),
            "rail leaf id never collides with a root id"
        );

        // It is NOT in the widget-rendered root tree.
        assert!(
            state.root().find_node(rail_id).is_none(),
            "rail leaf is excluded from the root tree the widget lays out"
        );
        // But it IS in the rail tree.
        assert!(
            state.rail().find_node(rail_id).is_some(),
            "rail leaf lives in the sibling rail grid"
        );
    }

    #[test]
    fn remove_nested_node() {
        let mut state: State<&str> = State::new(12);
        let group = state.add_group([0, 0, 6, 4], Width::Shrink, "group");
        let child = state.add_child(group, [0, 0, 2, 2], "child").unwrap();

        assert_eq!(state.remove(child), Some("child"));
        assert!(state.get(child).is_none());
        // The group itself survives.
        assert!(state.get_node(group).is_some());
    }

    // ── reparent (cross-group transfer) tests ───────────────────

    /// Builds a state with two sibling groups, each holding one child.
    fn two_groups() -> (State<&'static str>, ItemId, ItemId, ItemId, ItemId) {
        let mut state: State<&str> = State::new(12);
        let pulse = state.add_group([0, 0, 6, 4], Width::Shrink, "pulse");
        let trends = state.add_group([6, 0, 6, 4], Width::Shrink, "trends");
        let a = state.add_child(pulse, [0, 0, 2, 2], "a").unwrap();
        let b = state.add_child(trends, [0, 0, 2, 2], "b").unwrap();
        (state, pulse, trends, a, b)
    }

    fn reparent(
        node: ItemId,
        new_parent: Option<ItemId>,
        x: u16,
        y: u16,
        phase: DragPhase,
    ) -> Action {
        Action::Reparent {
            node,
            new_parent,
            x,
            y,
            phase,
            mode: MoveMode::Swap,
        }
    }

    #[test]
    fn reparent_deferred_until_ended() {
        let (mut state, _pulse, trends, a, _b) = two_groups();

        for phase in [DragPhase::Started, DragPhase::Ongoing] {
            state.perform(reparent(a, Some(trends), 2, 0, phase), |_, _| false);
            // Still in its original group, untouched.
            assert_eq!(state.get(a), Some(&"a"));
            assert_eq!(state.engine_of(a).unwrap().items().count(), 1);
        }
    }

    #[test]
    fn reparent_moves_node_across_groups_keeping_id() {
        let (mut state, pulse, trends, a, _b) = two_groups();

        state.perform(
            reparent(a, Some(trends), 2, 0, DragPhase::Ended),
            |_, _| false,
        );

        // Same id, now living in `trends` alongside `b`.
        assert_eq!(state.get(a), Some(&"a"));
        let trends_grid = state.get_node(trends).unwrap();
        assert!(
            trends_grid
                .children
                .as_ref()
                .unwrap()
                .find_node(a)
                .is_some()
        );

        // Source group `pulse` is now empty.
        let pulse_grid = state.get_node(pulse).unwrap();
        assert!(pulse_grid.children.as_ref().unwrap().is_empty());
    }

    #[test]
    fn reparent_to_root() {
        let (mut state, _pulse, _trends, a, _b) = two_groups();
        let root_before = state.len();

        state.perform(reparent(a, None, 0, 3, DragPhase::Ended), |_, _| false);

        // `a` is now a root-level node.
        assert!(state.iter().any(|(id, _)| id == a));
        assert_eq!(state.len(), root_before + 1);
    }

    #[test]
    fn reparent_into_own_subtree_is_noop() {
        // Dropping a group into its own subtree would create a cycle and
        // must be rejected, leaving the tree untouched.
        let mut state: State<&str> = State::new(12);
        let outer = state.add_group([0, 0, 12, 6], Width::Shrink, "outer");
        let inner = state.add_group([0, 0, 6, 4], Width::Shrink, "inner");
        // Nest `inner` under `outer` first.
        state.perform(
            reparent(inner, Some(outer), 0, 0, DragPhase::Ended),
            |_, _| false,
        );

        // outer → inner (its own descendant): rejected.
        state.perform(
            reparent(outer, Some(inner), 0, 0, DragPhase::Ended),
            |_, _| false,
        );
        // outer stays at the root; inner stays under outer.
        assert!(state.iter().any(|(id, _)| id == outer));
        let outer_grid =
            state.get_node(outer).unwrap().children.as_ref().unwrap();
        assert!(outer_grid.find_node(inner).is_some());

        // outer → outer (itself): also rejected.
        state.perform(
            reparent(outer, Some(outer), 0, 0, DragPhase::Ended),
            |_, _| false,
        );
        assert!(state.iter().any(|(id, _)| id == outer));
    }

    #[test]
    fn reparent_preserves_moved_subtree() {
        // Moving a container carries its children along.
        let mut state: State<&str> = State::new(12);
        let host = state.add_group([0, 0, 12, 8], Width::Shrink, "host");
        let group = state.add_group([0, 0, 6, 4], Width::Shrink, "group");
        let child = state.add_child(group, [0, 0, 2, 2], "child").unwrap();

        // Move `group` (with `child`) into `host`.
        state.perform(
            reparent(group, Some(host), 0, 0, DragPhase::Ended),
            |_, _| false,
        );

        // `child` still reachable, still under `group`, now under `host`.
        assert_eq!(state.get(child), Some(&"child"));
        let host_grid =
            state.get_node(host).unwrap().children.as_ref().unwrap();
        assert!(host_grid.find_node(group).is_some());
        assert!(host_grid.find_node(child).is_some());
    }

    #[test]
    fn shrink_group_lets_children_grow_past_its_authored_width() {
        // A Shrink group authored 4 wide: its inner grid spans the root's 12
        // columns, so a child can be resized wider than 4, and the group grows
        // to match (size-to-content). A Fixed group would clamp at 4.
        let mut state: State<&str> = State::new(12);
        state.fit(true);
        let g = state.add_group([0, 0, 4, 1], Width::Shrink, "g");
        let tile = state.add_child(g, [0, 0, 4, 3], "t").unwrap();
        assert_eq!(
            state.get_item(g).unwrap().w,
            4,
            "starts at content width 4"
        );

        // Resize the child to 6 — past the group's authored 4.
        assert!(state.resize_item(tile, 6, 3));
        assert_eq!(
            state.get_item(tile).unwrap().w,
            6,
            "child grows past the authored width (not clamped to 4)"
        );
        assert_eq!(
            state.get_item(g).unwrap().w,
            6,
            "the group grows to fit its content"
        );

        // A Fixed group, by contrast, clamps the child at its width.
        let fixed = state.add_group([0, 6, 4, 1], 4, "f");
        let fchild = state.add_child(fixed, [0, 0, 6, 3], "ft").unwrap();
        assert_eq!(
            state.get_item(fchild).unwrap().w,
            4,
            "Fixed group caps the child at its 4 columns"
        );
    }

    #[test]
    fn reparent_restores_width_clamped_by_a_narrow_grid() {
        let mut state: State<&str> = State::new(12);
        // A Fixed single-column group (its inner grid is exactly 1 column)
        // inside the 12-column root. (A Shrink group wouldn't clamp — its inner
        // grid spans the root's columns — so this exercises Fixed.)
        let narrow = state.add_group([0, 0, 1, 6], 1, "narrow");
        // Author a 2-wide tile in it; the 1-column grid clamps it to 1, but
        // the node remembers it wants to be 2 wide.
        let tile = state.add_child(narrow, [0, 0, 2, 2], "t").unwrap();
        {
            let n = state.engine_of(tile).unwrap().get(tile).unwrap();
            assert_eq!(n.w, 1, "clamped to the group's single column");
            assert_eq!(n.desired_w, 2, "but remembers its authored width");
        }

        // Drag it out to the 12-column root, which has room.
        state.perform(reparent(tile, None, 0, 7, DragPhase::Ended), |_, _| {
            false
        });

        let n = state.engine().get(tile).unwrap();
        assert_eq!(n.w, 2, "re-expands to its desired width in the wider grid");
        assert_eq!(n.desired_w, 2);
    }
}
