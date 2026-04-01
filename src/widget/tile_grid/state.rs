//! User-facing state for the tile grid.
//!
//! [`State`] wraps an [`Internal`] layout engine and associates user data
//! of type `T` with each grid item. It provides a convenient API for
//! managing items and their associated data together.

use std::collections::BTreeMap;

use super::ItemId;
use super::engine::{GridItem, Internal};
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
    pub fn get_item(&self, id: ItemId) -> Option<&GridItem> {
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
                id, x, y, phase, ..
            } => {
                let held = self.held_ids(&is_held);
                match phase {
                    DragPhase::Started => {
                        self.internal.begin_batch();
                        self.internal.move_item_held(id, x, y, &held);
                    }
                    DragPhase::Ongoing => {
                        self.internal.move_item_held(id, x, y, &held);
                    }
                    DragPhase::Ended => {
                        self.internal.move_item_held(id, x, y, &held);
                        self.internal.end_batch();
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
