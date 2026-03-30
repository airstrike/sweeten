//! User-facing state for the grid stack.
//!
//! [`State`] wraps a [`GridEngine`] and associates user data of type `T`
//! with each grid item. It provides a convenient API for managing items
//! and their associated data together.

use std::collections::BTreeMap;

use super::ItemId;
use super::engine::{GridEngine, GridItem};

/// User-facing state that pairs a [`GridEngine`] with user data for each item.
///
/// This is analogous to iced's `pane_grid::State<T>`, which pairs a layout
/// tree with a `BTreeMap<Pane, T>`.
///
/// # Example
///
/// ```
/// use sweeten::widget::grid_stack::State;
///
/// let mut state: State<String> = State::new(12);
///
/// let id = state.add(0, 0, 4, 2, "Widget A".to_string());
/// assert_eq!(state.get(id), Some(&"Widget A".to_string()));
///
/// state.move_item(id, 4, 0);
/// state.remove(id);
/// ```
#[derive(Debug, Clone)]
pub struct State<T> {
    engine: GridEngine,
    data: BTreeMap<ItemId, T>,
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
            engine: GridEngine::new(columns),
            data: BTreeMap::new(),
        }
    }

    /// Returns a reference to the underlying [`GridEngine`].
    #[must_use]
    pub fn engine(&self) -> &GridEngine {
        &self.engine
    }

    /// Returns a mutable reference to the underlying [`GridEngine`].
    pub fn engine_mut(&mut self) -> &mut GridEngine {
        &mut self.engine
    }

    /// Returns the number of columns in the grid.
    #[must_use]
    pub fn columns(&self) -> u16 {
        self.engine.columns()
    }

    /// Sets float mode on the engine.
    pub fn set_float(&mut self, float: bool) {
        self.engine.set_float(float);
    }

    /// Returns whether float mode is enabled.
    #[must_use]
    pub fn float(&self) -> bool {
        self.engine.float()
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
        let id = self.engine.add_item(x, y, w, h);
        self.data.insert(id, user_data);
        id
    }

    /// Adds a new item with auto-placement.
    ///
    /// The engine finds the first empty position that fits the given size.
    /// Returns `None` if the grid is full (only possible when `max_rows`
    /// is set).
    pub fn add_auto(&mut self, w: u16, h: u16, user_data: T) -> Option<ItemId> {
        let id = self.engine.add_item_auto(w, h)?;
        self.data.insert(id, user_data);
        Some(id)
    }

    /// Removes an item and returns its associated data.
    ///
    /// Returns `None` if no item with the given ID exists.
    pub fn remove(&mut self, id: ItemId) -> Option<T> {
        self.engine.remove_item(id)?;
        self.data.remove(&id)
    }

    /// Moves an item to a new grid position.
    ///
    /// Returns `true` if the item was actually moved.
    pub fn move_item(&mut self, id: ItemId, x: u16, y: u16) -> bool {
        self.engine.move_item(id, x, y)
    }

    /// Resizes an item.
    ///
    /// Returns `true` if the item was actually resized.
    pub fn resize_item(&mut self, id: ItemId, w: u16, h: u16) -> bool {
        self.engine.resize_item(id, w, h)
    }

    /// Returns a reference to the user data for the given item.
    #[must_use]
    pub fn get(&self, id: ItemId) -> Option<&T> {
        self.data.get(&id)
    }

    /// Returns a mutable reference to the user data for the given item.
    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut T> {
        self.data.get_mut(&id)
    }

    /// Returns the grid item (position/size) for the given ID.
    #[must_use]
    pub fn get_item(&self, id: ItemId) -> Option<&GridItem> {
        self.engine.get(id)
    }

    /// Returns an iterator over all `(ItemId, &T)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (ItemId, &T)> {
        self.data.iter().map(|(&id, data)| (id, data))
    }

    /// Returns an iterator over all `(ItemId, &mut T)` pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ItemId, &mut T)> {
        self.data.iter_mut().map(|(&id, data)| (id, data))
    }

    /// Returns the number of items in the grid.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the grid has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the current grid height (max bottom edge of any item).
    #[must_use]
    pub fn get_row(&self) -> u16 {
        self.engine.get_row()
    }

    /// Converts all items to pixel rectangles.
    ///
    /// See [`GridEngine::item_regions`] for details.
    #[must_use]
    pub fn item_regions(
        &self,
        bounds: (f32, f32),
        spacing: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        self.engine.item_regions(bounds, spacing)
    }
}
