//! User-facing state for the tile grid.
//!
//! [`State`] wraps an [`Internal`] layout engine and associates user data
//! of type `T` with each grid item. It provides a convenient API for
//! managing items and their associated data together.

use std::collections::BTreeMap;

use super::ItemId;
use super::configuration::Configuration;
use super::engine::{Internal, Node};
use super::widget::{Action, DragPhase};

/// User-facing state that pairs an [`Internal`] layout engine with user
/// data for each item.
///
/// This is analogous to iced's [`pane_grid::State<T>`], which pairs a
/// layout tree with a `BTreeMap<Pane, T>`.
///
/// # Example
///
/// ```
/// use sweeten::widget::tile_grid::State;
///
/// let mut state: State<String> = State::new(12);
///
/// let id = state.add(0, 0, 4, 2, "Widget A".to_string());
/// assert_eq!(state.get(id), Some(&"Widget A".to_string()));
///
/// state.remove(id);
/// ```
///
/// [`pane_grid::State<T>`]: https://docs.iced.rs/iced/widget/pane_grid/state/struct.State.html
#[derive(Debug, Clone)]
pub struct State<T> {
    /// The items and their user data.
    ///
    /// Each entry maps an [`ItemId`] to the application-specific data
    /// associated with that grid item.
    pub items: BTreeMap<ItemId, T>,

    /// The internal layout state.
    ///
    /// Contains the grid engine that manages item positions, sizes, and
    /// collision resolution.
    pub internal: Internal,
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
            internal: Internal::new(columns),
            items: BTreeMap::new(),
        }
    }

    /// Creates a new [`State`] from the given [`Configuration`].
    ///
    /// Items are assigned monotonic [`ItemId`]s in the order they appear
    /// in the configuration. Collisions are resolved during construction;
    /// gravity compaction runs once at the end (unless `float` is
    /// enabled).
    ///
    /// # Example
    ///
    /// ```
    /// use sweeten::widget::tile_grid::{State, Configuration};
    ///
    /// let state: State<&str> = State::with_configuration(
    ///     Configuration::new(12)
    ///         .with_item(0, 0, 4, 2, "left")
    ///         .with_item(4, 0, 8, 2, "right"),
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
        state.internal.set_max_rows(config.max_rows);

        // Batch mode defers gravity compaction until all items are added,
        // avoiding O(n) intermediate pack_nodes passes.
        state.internal.begin_batch();
        if config.float {
            state.internal.set_float(true);
        }

        for item in config.items {
            let id = state.add(item.x, item.y, item.w, item.h, item.state);

            let has_constraints = item.min_w.is_some()
                || item.max_w.is_some()
                || item.min_h.is_some()
                || item.max_h.is_some();

            if has_constraints {
                state.internal.set_item_constraints(
                    id, item.min_w, item.max_w, item.min_h, item.max_h,
                );
            }
        }

        // Single gravity pass (no-op when float is enabled).
        state.internal.end_batch();
        state
    }

    /// Returns the number of columns in the grid.
    #[must_use]
    pub fn columns(&self) -> u16 {
        self.internal.columns()
    }

    /// Sets float mode on the engine.
    pub fn set_float(&mut self, float: bool) {
        self.internal.set_float(float);
    }

    /// Returns whether float mode is enabled.
    #[must_use]
    pub fn float(&self) -> bool {
        self.internal.float()
    }

    /// Adds a new item at the given grid position with associated data.
    ///
    /// Returns the [`ItemId`] of the newly created item.
    pub fn add(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        user_data: T,
    ) -> ItemId {
        let id = self.internal.add_item(x, y, w, h);
        self.items.insert(id, user_data);
        id
    }

    /// Adds a new item with auto-placement.
    ///
    /// The engine finds the first empty position that fits the given size.
    /// Returns `None` if the grid is full (only possible when `max_rows`
    /// is set).
    pub fn add_auto(&mut self, w: u16, h: u16, user_data: T) -> Option<ItemId> {
        let id = self.internal.add_item_auto(w, h)?;
        self.items.insert(id, user_data);
        Some(id)
    }

    /// Removes an item and returns its associated data.
    ///
    /// Returns `None` if no item with the given ID exists.
    pub fn remove(&mut self, id: ItemId) -> Option<T> {
        self.internal.remove_item(id)?;
        self.items.remove(&id)
    }

    /// Moves an item to a new grid position.
    ///
    /// Returns `true` if the item was actually moved.
    pub fn move_item(&mut self, id: ItemId, x: u16, y: u16) -> bool {
        self.internal.move_item(id, x, y)
    }

    /// Resizes an item.
    ///
    /// Returns `true` if the item was actually resized.
    pub fn resize_item(&mut self, id: ItemId, w: u16, h: u16) -> bool {
        self.internal.resize_item(id, w, h)
    }

    /// Returns a reference to the user data for the given item.
    #[must_use]
    pub fn get(&self, id: ItemId) -> Option<&T> {
        self.items.get(&id)
    }

    /// Returns a mutable reference to the user data for the given item.
    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut T> {
        self.items.get_mut(&id)
    }

    /// Returns the grid item (position/size) for the given ID.
    #[must_use]
    pub fn get_item(&self, id: ItemId) -> Option<&Node> {
        self.internal.get(id)
    }

    /// Returns an iterator over all `(ItemId, &T)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (ItemId, &T)> {
        self.items.iter().map(|(&id, data)| (id, data))
    }

    /// Returns an iterator over all `(ItemId, &mut T)` pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ItemId, &mut T)> {
        self.items.iter_mut().map(|(&id, data)| (id, data))
    }

    /// Returns the number of items in the grid.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the grid has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the current grid height (max bottom edge of any item).
    #[must_use]
    pub fn get_row(&self) -> u16 {
        self.internal.get_row()
    }

    /// Converts all items to pixel rectangles.
    ///
    /// See [`Internal::item_regions`] for details.
    #[must_use]
    pub fn item_regions(
        &self,
        bounds: (f32, f32),
        spacing: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        self.internal.item_regions(bounds, spacing)
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
    /// Move and resize actions are applied to the engine, with batch mode
    /// managed automatically for resize operations.
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
                        // Apply the move for real using the mode
                        // resolved by the widget at drop time.
                        let held = self.held_ids(&is_held);
                        self.internal.save_snapshot();
                        self.internal.move_item_held(id, x, y, &held, mode);
                        self.internal.clear_snapshot();
                    }
                }
            }
            Action::Resize {
                id, w, h, phase, ..
            } => {
                let held = self.held_ids(&is_held);
                match phase {
                    DragPhase::Started => {
                        self.internal.begin_batch();
                        self.internal.resize_item_held(id, w, h, &held);
                    }
                    DragPhase::Ongoing => {
                        self.internal.resize_item_held(id, w, h, &held);
                    }
                    DragPhase::Ended => {
                        self.internal.resize_item_held(id, w, h, &held);
                        self.internal.end_batch();
                    }
                }
            }
        }
    }

    /// Collects the IDs of items for which the `is_held` predicate
    /// returns `true`.
    fn held_ids(&self, is_held: &impl Fn(ItemId, &T) -> bool) -> Vec<ItemId> {
        self.items
            .iter()
            .filter(|&(&id, data)| is_held(id, data))
            .map(|(&id, _)| id)
            .collect()
    }
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
        let a = state.add(0, 0, 4, 2, ());
        let _b = state.add(4, 0, 4, 2, ());
        let _c = state.add(8, 0, 4, 2, ());

        // Snapshot initial positions.
        let initial: Vec<_> =
            state.internal.items().map(|i| (i.id, i.x, i.y)).collect();

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
            state.internal.items().map(|i| (i.id, i.x, i.y)).collect();
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
            state.internal.items().map(|i| (i.id, i.x, i.y)).collect();
        assert_eq!(
            initial, after_ongoing,
            "engine must not change on DragPhase::Ongoing"
        );
    }

    #[test]
    fn perform_move_ended_applies_with_swap() {
        // On DragPhase::Ended, the engine should apply the move
        // including swap logic (same-size items swap positions).
        let mut state: State<()> = State::new(12);
        let a = state.add(0, 0, 4, 2, ());
        let b = state.add(4, 0, 4, 2, ());
        state.add(8, 0, 4, 2, ());

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
        assert_eq!(state.internal.max_rows(), Some(5));
        assert!(state.float());
    }

    #[test]
    fn with_configuration_monotonic_ids() {
        let state: State<&str> = State::with_configuration(
            Configuration::new(12)
                .with_item(0, 0, 4, 2, "a")
                .with_item(4, 0, 4, 2, "b")
                .with_item(8, 0, 4, 2, "c"),
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
                .with_item(0, 0, 4, 2, ())
                .with_item(4, 0, 4, 2, ())
                .with_item(8, 0, 4, 2, ()),
        );

        let mut positions: Vec<_> = state
            .internal
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
            Configuration::new(12).with_item(0, 0, 4, 2, ()).with_item(
                0,
                0,
                4,
                2,
                (),
            ),
        );

        let positions: Vec<_> =
            state.internal.items().map(|n| (n.x, n.y)).collect();

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
            Configuration::new(12).with_item(0, 10, 4, 2, ()).with_item(
                4,
                10,
                4,
                2,
                (),
            ),
        );

        for node in state.internal.items() {
            assert_eq!(node.y, 0, "gravity should compact items to y=0");
        }
    }

    #[test]
    fn with_configuration_float_preserves_y() {
        // With float enabled, items should stay at their declared y.
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .float(true)
                .with_item(0, 10, 4, 2, ())
                .with_item(4, 10, 4, 2, ()),
        );

        for node in state.internal.items() {
            assert_eq!(node.y, 10, "float mode should preserve declared y");
        }
    }

    #[test]
    fn with_configuration_constraints_applied() {
        let state: State<()> = State::with_configuration(
            Configuration::new(12)
                .push(Item::new(0, 0, 2, 1, ()).min_w(4).min_h(3)),
        );

        let node = state.internal.items().next().unwrap();
        assert!(node.w >= 4, "min_w constraint should be applied");
        assert!(node.h >= 3, "min_h constraint should be applied");
    }

    #[test]
    fn with_configuration_max_rows_honored() {
        // An item extending past max_rows should be clamped.
        let state: State<()> = State::with_configuration(
            Configuration::new(12).max_rows(5).float(true).with_item(
                0,
                0,
                4,
                10,
                (),
            ),
        );

        let node = state.internal.items().next().unwrap();
        assert!(node.y + node.h <= 5, "item should not extend past max_rows");
    }

    #[test]
    fn with_configuration_user_data_accessible() {
        let state: State<&str> = State::with_configuration(
            Configuration::new(12)
                .with_item(0, 0, 6, 2, "hello")
                .with_item(6, 0, 6, 2, "world"),
        );

        let mut values: Vec<_> = state.iter().map(|(_, data)| *data).collect();
        values.sort();
        assert_eq!(values, vec!["hello", "world"]);
    }
}
