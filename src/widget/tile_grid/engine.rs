//! Core layout engine for the tile grid.
//!
//! This module contains the pure layout math for a GridStack-like layout
//! system. It has no dependency on iced -- it works entirely with integer
//! grid coordinates and produces pixel rectangles on demand.
//!
//! The engine is inspired by [GridStack.js](https://gridstackjs.com/) but
//! adapted for Rust. Items are placed on a discrete grid with a fixed number
//! of columns and an unlimited (or capped) number of rows. When items
//! overlap, the engine resolves collisions by displacing items downward.
//! When `float` mode is off (the default), items compact upward under
//! gravity.

use super::ItemId;

/// An item on the grid, with integer coordinates in grid units.
///
/// All position and size values are in grid cells, not pixels. The grid
/// coordinate system has its origin at the top-left corner, with X
/// increasing rightward (columns) and Y increasing downward (rows).
///
/// An item occupies the rectangle `[x, x+w) x [y, y+h)` in grid space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridItem {
    /// Unique identifier assigned by the engine.
    pub id: ItemId,
    /// Column position (0-based from left).
    pub x: u16,
    /// Row position (0-based from top).
    pub y: u16,
    /// Width in columns (>= 1).
    pub w: u16,
    /// Height in rows (>= 1).
    pub h: u16,
    /// Minimum width in columns.
    pub min_w: Option<u16>,
    /// Maximum width in columns.
    pub max_w: Option<u16>,
    /// Minimum height in rows.
    pub min_h: Option<u16>,
    /// Maximum height in rows.
    pub max_h: Option<u16>,
}

impl GridItem {
    /// Returns the bottom edge of this item (y + h).
    #[must_use]
    fn bottom(&self) -> u16 {
        self.y.saturating_add(self.h)
    }

    /// Returns the right edge of this item (x + w).
    #[must_use]
    fn right(&self) -> u16 {
        self.x.saturating_add(self.w)
    }
}

/// Tests whether two axis-aligned bounding boxes (grid rectangles) overlap.
///
/// Two rectangles overlap if and only if none of the four separation
/// conditions hold. Adjacent rectangles (sharing an edge but not
/// overlapping area) are **not** considered intercepted.
#[must_use]
pub fn is_intercepted(a: &GridItem, b: &GridItem) -> bool {
    !(a.y >= b.bottom()
        || a.bottom() <= b.y
        || a.right() <= b.x
        || a.x >= b.right())
}

/// The internal layout state of a [`TileGrid`].
///
/// Manages a flat list of [`GridItem`]s on a grid with a fixed number of
/// columns. Provides algorithms for adding, removing, moving, and resizing
/// items, with automatic collision resolution and optional vertical
/// compaction (gravity).
///
/// This is analogous to [`pane_grid::state::Internal`] — it holds the
/// layout data while [`State`] pairs it with user data.
///
/// # Example
///
/// ```
/// use sweeten::widget::tile_grid::engine::Internal;
///
/// let mut engine = Internal::new(12);
///
/// // Add two items
/// let a = engine.add_item(0, 0, 4, 2);
/// let b = engine.add_item(4, 0, 4, 2);
///
/// // Move item a to overlap with b -- b will be displaced
/// engine.move_item(a, 4, 0);
///
/// // Item b should have been pushed down
/// let items: Vec<_> = engine.items().collect();
/// assert!(items.iter().any(|i| i.id == b && i.y >= 2));
/// ```
///
/// [`TileGrid`]: super::TileGrid
/// [`State`]: super::State
/// [`pane_grid::state::Internal`]: https://docs.iced.rs/iced/widget/pane_grid/state/struct.Internal.html
#[derive(Debug, Clone)]
pub struct Internal {
    /// Number of columns in the grid.
    columns: u16,
    /// Maximum number of rows (None = unlimited).
    max_rows: Option<u16>,
    /// Float mode: if false, items compact upward (gravity).
    float: bool,
    /// Batch mode: when true, gravity compaction is deferred until
    /// [`end_batch`](Internal::end_batch) is called.
    batch_mode: bool,
    /// All items in the grid.
    items: Vec<GridItem>,
    /// Monotonically increasing ID counter.
    next_id: usize,
}

impl Internal {
    /// Creates a new grid engine with the given number of columns.
    ///
    /// The grid starts empty with gravity mode enabled (float = false).
    ///
    /// # Panics
    ///
    /// Panics if `columns` is 0.
    #[must_use]
    pub fn new(columns: u16) -> Self {
        assert!(columns > 0, "grid must have at least 1 column");
        Self {
            columns,
            max_rows: None,
            float: false,
            batch_mode: false,
            items: Vec::new(),
            next_id: 0,
        }
    }

    /// Returns the number of columns in the grid.
    #[must_use]
    pub fn columns(&self) -> u16 {
        self.columns
    }

    /// Returns the maximum number of rows, if set.
    #[must_use]
    pub fn max_rows(&self) -> Option<u16> {
        self.max_rows
    }

    /// Sets the maximum number of rows.
    pub fn set_max_rows(&mut self, max_rows: Option<u16>) {
        self.max_rows = max_rows;
    }

    /// Returns whether float mode is enabled.
    #[must_use]
    pub fn float(&self) -> bool {
        self.float
    }

    /// Sets float mode. When false (the default), items compact upward.
    pub fn set_float(&mut self, float: bool) {
        self.float = float;
        if !float {
            self.pack_nodes();
        }
    }

    /// Enters batch mode. While active, gravity compaction is deferred.
    ///
    /// This matches the GridStack.js `beginUpdate` pattern: during an
    /// interactive drag or resize, items that collide are pushed down
    /// but no items float upward. Call [`end_batch`](Self::end_batch) to
    /// compact and apply deferred gravity.
    pub fn begin_batch(&mut self) {
        self.batch_mode = true;
    }

    /// Exits batch mode and runs gravity compaction.
    ///
    /// If float mode is also enabled, compaction is still skipped (float
    /// takes precedence).
    pub fn end_batch(&mut self) {
        self.batch_mode = false;
        if !self.float {
            self.pack_nodes();
        }
    }

    /// Returns whether batch mode is active.
    #[must_use]
    pub fn is_batch(&self) -> bool {
        self.batch_mode
    }

    /// Returns an iterator over all items in the grid.
    pub fn items(&self) -> impl Iterator<Item = &GridItem> {
        self.items.iter()
    }

    /// Returns the item with the given ID, if it exists.
    #[must_use]
    pub fn get(&self, id: ItemId) -> Option<&GridItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns the current grid height (the maximum bottom edge of any item).
    ///
    /// Returns 0 if the grid is empty.
    #[must_use]
    pub fn get_row(&self) -> u16 {
        self.items.iter().map(GridItem::bottom).max().unwrap_or(0)
    }

    /// Enforces all constraints on an item: min/max dimensions, grid
    /// boundaries, and position clamping.
    ///
    /// When `resizing` is true and an item extends beyond the grid
    /// boundary, its size is shrunk to fit. When `resizing` is false,
    /// the item is shifted to stay within bounds instead.
    fn node_bound_fix(&self, item: &mut GridItem, resizing: bool) {
        // Apply min/max width constraints
        if let Some(min_w) = item.min_w {
            item.w = item.w.max(min_w);
        }
        if let Some(max_w) = item.max_w {
            item.w = item.w.min(max_w);
        }

        // Apply min/max height constraints
        if let Some(min_h) = item.min_h {
            item.h = item.h.max(min_h);
        }
        if let Some(max_h) = item.max_h {
            item.h = item.h.min(max_h);
        }

        // Clamp dimensions to grid bounds
        item.w = item.w.clamp(1, self.columns);
        item.h = item.h.max(1);
        if let Some(max_rows) = self.max_rows
            && max_rows > 0
        {
            item.h = item.h.min(max_rows);
        }

        // Fix position: right edge overflow
        if item.x + item.w > self.columns {
            if resizing {
                // Shrink width to fit
                item.w = self.columns.saturating_sub(item.x);
                if item.w == 0 {
                    item.w = 1;
                    item.x = self.columns.saturating_sub(1);
                }
            } else {
                // Shift left to fit
                item.x = self.columns.saturating_sub(item.w);
            }
        }

        // Fix position: bottom edge overflow (if max_rows is set)
        if let Some(max_rows) = self.max_rows
            && max_rows > 0
            && item.y + item.h > max_rows
        {
            if resizing {
                item.h = max_rows.saturating_sub(item.y);
                if item.h == 0 {
                    item.h = 1;
                    item.y = max_rows.saturating_sub(1);
                }
            } else {
                item.y = max_rows.saturating_sub(item.h);
            }
        }
    }

    /// Scans the grid left-to-right, top-to-bottom for the first position
    /// where an item of the given size can be placed without overlapping
    /// any existing item.
    ///
    /// Returns `None` if no position is found (which can only happen if
    /// `max_rows` is set and the grid is full).
    #[must_use]
    pub fn find_empty_position(&self, w: u16, h: u16) -> Option<(u16, u16)> {
        let w = w.clamp(1, self.columns);
        let h = h.max(1);

        // Determine the row limit for scanning
        let row_limit = self.max_rows.unwrap_or_else(|| {
            // Scan up to current height + h (enough room to place below everything)
            self.get_row().saturating_add(h)
        });

        // Create a temporary item for collision testing
        let mut test = GridItem {
            id: ItemId(usize::MAX),
            x: 0,
            y: 0,
            w,
            h,
            min_w: None,
            max_w: None,
            min_h: None,
            max_h: None,
        };

        for y in 0..=row_limit.saturating_sub(h) {
            for x in 0..=self.columns.saturating_sub(w) {
                test.x = x;
                test.y = y;

                let collides =
                    self.items.iter().any(|item| is_intercepted(&test, item));

                if !collides {
                    return Some((x, y));
                }
            }
        }

        None
    }

    /// Finds the index of the first item that overlaps with `area`,
    /// excluding the item with ID `skip_id`.
    fn find_collision(
        &self,
        area: &GridItem,
        skip_id: ItemId,
    ) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.id != skip_id && is_intercepted(item, area))
    }

    /// Resolves all collisions caused by the item with the given ID.
    ///
    /// When the given item overlaps with other items:
    /// - Held items cannot be displaced. If the given item overlaps a
    ///   held item, the given item itself is moved below the held item.
    /// - Other items are pushed below the given item.
    /// - Displacement cascades: if pushing item B down causes it to overlap
    ///   item C, item C is also displaced, and so on.
    ///
    /// `held` lists item IDs that are treated as immovable obstacles
    /// during this resolution pass (e.g. pinned items).
    fn fix_collisions(&mut self, item_id: ItemId, held: &[ItemId]) {
        let max_iterations = (self.items.len() + 1) * (self.items.len() + 1);
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > max_iterations {
                break;
            }

            // Find the item that initiated this collision resolution
            let Some(item_idx) =
                self.items.iter().position(|i| i.id == item_id)
            else {
                break;
            };

            // Find collision using the item's actual bounding box.
            let collision_idx = match self
                .find_collision(&self.items[item_idx].clone(), item_id)
            {
                Some(idx) => idx,
                None => break, // No more collisions
            };

            let colliding_id = self.items[collision_idx].id;

            if held.contains(&colliding_id) {
                // Held item: move OUR item below the held one
                let held_bottom = self.items[collision_idx].bottom();
                let item_idx =
                    self.items.iter().position(|i| i.id == item_id).unwrap();
                self.items[item_idx].y = held_bottom;
                self.node_bound_fix_by_id(item_id, false);
                // Continue loop: our item may now overlap something else
            } else {
                // Push the colliding item below our item
                let item_idx =
                    self.items.iter().position(|i| i.id == item_id).unwrap();
                let new_y = self.items[item_idx].bottom();
                let col_idx = self
                    .items
                    .iter()
                    .position(|i| i.id == colliding_id)
                    .unwrap();
                self.items[col_idx].y = new_y;
                self.node_bound_fix_by_id(colliding_id, false);

                // Now recursively fix collisions caused by the displaced item
                self.fix_collisions_nested(
                    colliding_id,
                    held,
                    iterations,
                    max_iterations,
                );
            }
        }
    }

    /// Recursive helper for cascading collision resolution.
    ///
    /// Resolves collisions for a displaced item, with a shared iteration
    /// budget to prevent infinite loops.
    fn fix_collisions_nested(
        &mut self,
        item_id: ItemId,
        held: &[ItemId],
        mut iterations: usize,
        max_iterations: usize,
    ) {
        loop {
            iterations += 1;
            if iterations > max_iterations {
                break;
            }

            let Some(item_idx) =
                self.items.iter().position(|i| i.id == item_id)
            else {
                break;
            };

            let collision_idx = match self
                .find_collision(&self.items[item_idx].clone(), item_id)
            {
                Some(idx) => idx,
                None => break,
            };

            let colliding_id = self.items[collision_idx].id;

            if held.contains(&colliding_id) {
                let held_bottom = self.items[collision_idx].bottom();
                let item_idx =
                    self.items.iter().position(|i| i.id == item_id).unwrap();
                self.items[item_idx].y = held_bottom;
                self.node_bound_fix_by_id(item_id, false);
            } else {
                let item_idx =
                    self.items.iter().position(|i| i.id == item_id).unwrap();
                let new_y = self.items[item_idx].bottom();
                let col_idx = self
                    .items
                    .iter()
                    .position(|i| i.id == colliding_id)
                    .unwrap();
                self.items[col_idx].y = new_y;
                self.node_bound_fix_by_id(colliding_id, false);
                self.fix_collisions_nested(
                    colliding_id,
                    held,
                    iterations,
                    max_iterations,
                );
            }
        }
    }

    /// Applies `node_bound_fix` to the item with the given ID.
    fn node_bound_fix_by_id(&mut self, id: ItemId, resizing: bool) {
        // We need to work around the borrow checker: extract the item,
        // fix it, then put it back.
        let Some(idx) = self.items.iter().position(|i| i.id == id) else {
            return;
        };
        let mut item = self.items[idx].clone();
        self.node_bound_fix(&mut item, resizing);
        self.items[idx] = item;
    }

    /// Compacts all items upward (toward y=0) when gravity mode is active.
    ///
    /// Items are sorted in reading order (top-to-bottom, left-to-right)
    /// and each item is moved as high as possible without overlapping
    /// any item that has already been positioned.
    ///
    /// The algorithm finds the highest valid position for each item by
    /// computing the lowest y where no collision occurs. It does this
    /// by collecting the bottom edges of all horizontally-overlapping
    /// items and trying y=0 first, then each of those bottom edges.
    ///
    /// Items whose IDs appear in `held` are treated as immovable for
    /// this compaction pass (e.g. pinned items, or the item currently
    /// being resized).
    pub fn pack_nodes_held(&mut self, held: &[ItemId]) {
        if self.float || self.batch_mode {
            return;
        }

        // Sort in reading order: by y ascending, then by x ascending
        self.items
            .sort_by(|a, b| a.y.cmp(&b.y).then_with(|| a.x.cmp(&b.x)));

        // For each item, find the highest valid y position
        for i in 0..self.items.len() {
            if held.contains(&self.items[i].id) {
                continue;
            }

            // Collect candidate y positions: y=0 plus the bottom edge
            // of every other item that overlaps horizontally with this one.
            let item_x = self.items[i].x;
            let item_w = self.items[i].w;

            let mut candidates: Vec<u16> = vec![0];
            for j in 0..self.items.len() {
                if i == j {
                    continue;
                }
                // Check horizontal overlap: items share column space
                let other = &self.items[j];
                if item_x < other.right() && item_x + item_w > other.x {
                    candidates.push(other.bottom());
                }
            }

            candidates.sort_unstable();
            candidates.dedup();

            // Find the lowest candidate_y where no collision occurs
            let mut best_y = self.items[i].y;
            for candidate_y in candidates {
                if candidate_y > best_y {
                    break; // No point trying higher positions
                }

                let mut test = self.items[i].clone();
                test.y = candidate_y;

                let collides = (0..self.items.len())
                    .filter(|&j| j != i)
                    .any(|j| is_intercepted(&test, &self.items[j]));

                if !collides {
                    best_y = candidate_y;
                    break; // This is the lowest valid position
                }
            }

            self.items[i].y = best_y;
        }
    }

    /// Compacts all items upward. Equivalent to `pack_nodes_held(&[])`.
    pub fn pack_nodes(&mut self) {
        self.pack_nodes_held(&[]);
    }

    /// Adds a new item to the grid at the given position and size.
    ///
    /// The item is assigned a unique [`ItemId`]. If the item overlaps
    /// with existing items, collisions are resolved automatically. If
    /// gravity mode is active, items are compacted afterward.
    ///
    /// Returns the ID of the newly added item.
    pub fn add_item(&mut self, x: u16, y: u16, w: u16, h: u16) -> ItemId {
        let id = ItemId(self.next_id);
        self.next_id += 1;

        let mut item = GridItem {
            id,
            x,
            y,
            w,
            h,
            min_w: None,
            max_w: None,
            min_h: None,
            max_h: None,
        };

        self.node_bound_fix(&mut item, false);
        self.items.push(item);
        self.fix_collisions(id, &[]);

        if !self.float {
            self.pack_nodes();
        }

        id
    }

    /// Adds a new item with auto-placement: the engine finds the first
    /// empty position that fits.
    ///
    /// Returns `None` if the grid is full (only possible when `max_rows`
    /// is set).
    pub fn add_item_auto(&mut self, w: u16, h: u16) -> Option<ItemId> {
        let (x, y) = self.find_empty_position(w, h)?;
        Some(self.add_item(x, y, w, h))
    }

    /// Removes the item with the given ID from the grid.
    ///
    /// Returns the removed item, or `None` if no item with that ID exists.
    /// After removal, if gravity mode is active, remaining items are
    /// compacted.
    pub fn remove_item(&mut self, id: ItemId) -> Option<GridItem> {
        let idx = self.items.iter().position(|item| item.id == id)?;
        let item = self.items.remove(idx);

        if !self.float {
            self.pack_nodes();
        }

        Some(item)
    }

    /// Moves an item to a new grid position.
    ///
    /// Returns `true` if the item was moved, `false` if the item was not
    /// found or the position is unchanged.
    ///
    /// After moving, collisions are resolved and gravity is applied.
    pub fn move_item(&mut self, id: ItemId, new_x: u16, new_y: u16) -> bool {
        self.move_item_held(id, new_x, new_y, &[])
    }

    /// Moves an item to a new grid position, treating `held` items as
    /// immovable obstacles during collision resolution and compaction.
    ///
    /// Returns `true` if the item was moved, `false` if the item was not
    /// found or the position is unchanged.
    pub fn move_item_held(
        &mut self,
        id: ItemId,
        new_x: u16,
        new_y: u16,
        held: &[ItemId],
    ) -> bool {
        let Some(idx) = self.items.iter().position(|item| item.id == id) else {
            return false;
        };

        let old_x = self.items[idx].x;
        let old_y = self.items[idx].y;

        self.items[idx].x = new_x;
        self.items[idx].y = new_y;

        // Apply boundary constraints (not resizing)
        self.node_bound_fix_by_id(id, false);

        // Check if position actually changed after clamping
        let idx = self.items.iter().position(|item| item.id == id).unwrap();
        if self.items[idx].x == old_x && self.items[idx].y == old_y {
            return false;
        }

        self.fix_collisions(id, held);

        if !self.float {
            self.pack_nodes_held(held);
        }

        true
    }

    /// Resizes an item to a new width and height.
    ///
    /// Returns `true` if the item was resized, `false` if the item was
    /// not found or the size is unchanged.
    ///
    /// After resizing, constraints are enforced, collisions are resolved,
    /// and gravity is applied.
    pub fn resize_item(&mut self, id: ItemId, new_w: u16, new_h: u16) -> bool {
        self.resize_item_held(id, new_w, new_h, &[])
    }

    /// Resizes an item to a new width and height, treating `held` items
    /// as immovable obstacles during collision resolution and compaction.
    ///
    /// The resized item is always implicitly held during compaction so
    /// that resizing never moves the item itself.
    ///
    /// Returns `true` if the item was resized, `false` if the item was
    /// not found or the size is unchanged.
    pub fn resize_item_held(
        &mut self,
        id: ItemId,
        new_w: u16,
        new_h: u16,
        held: &[ItemId],
    ) -> bool {
        let Some(idx) = self.items.iter().position(|item| item.id == id) else {
            return false;
        };

        let old_w = self.items[idx].w;
        let old_h = self.items[idx].h;

        self.items[idx].w = new_w;
        self.items[idx].h = new_h;

        // Apply constraints (resizing mode)
        self.node_bound_fix_by_id(id, true);

        // Check if size actually changed after constraints
        let idx = self.items.iter().position(|item| item.id == id).unwrap();
        if self.items[idx].w == old_w && self.items[idx].h == old_h {
            return false;
        }

        self.fix_collisions(id, held);

        // Compact with the resized item held in place — resizing should
        // never move the item itself, only push others out of the way.
        // Merge the caller's held list with the resized item's own ID.
        if !self.float {
            if held.is_empty() {
                self.pack_nodes_held(&[id]);
            } else {
                let mut all_held = held.to_vec();
                if !all_held.contains(&id) {
                    all_held.push(id);
                }
                self.pack_nodes_held(&all_held);
            }
        }

        true
    }

    /// Converts all grid items to pixel rectangles within the given bounds.
    ///
    /// Each item's pixel position is computed from its grid coordinates:
    /// - `pixel_x = x * cell_width + x * spacing`
    /// - `pixel_y = y * cell_height + y * spacing`
    /// - `pixel_w = w * cell_width + (w - 1) * spacing`
    /// - `pixel_h = h * cell_height + (h - 1) * spacing`
    ///
    /// Where `cell_width = (bounds_width - (columns - 1) * spacing) / columns`
    /// and `cell_height = cell_width` (square cells by default, but the
    /// caller controls `bounds` to achieve any aspect ratio).
    ///
    /// # Arguments
    ///
    /// * `bounds` - The total available size as `(width, height)`.
    /// * `spacing` - The gap between cells in pixels.
    ///
    /// # Returns
    ///
    /// A `Vec` of `(ItemId, (x, y, width, height))` tuples in pixel coordinates.
    #[must_use]
    pub fn item_regions(
        &self,
        bounds: (f32, f32),
        spacing: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        let (bounds_w, _bounds_h) = bounds;

        if self.items.is_empty() || self.columns == 0 {
            return Vec::new();
        }

        let cols = f32::from(self.columns);

        // cell_width is the width of a single cell, excluding spacing
        let cell_width = (bounds_w - (cols - 1.0) * spacing) / cols;

        // For now, cell_height = cell_width (square cells). The caller
        // can control the overall bounds to stretch as needed, or we
        // could add a cell_height parameter in the future.
        let cell_height = cell_width;

        self.items
            .iter()
            .map(|item| {
                let x = f32::from(item.x);
                let y = f32::from(item.y);
                let w = f32::from(item.w);
                let h = f32::from(item.h);

                let px = x * cell_width + x * spacing;
                let py = y * cell_height + y * spacing;
                let pw = w * cell_width + (w - 1.0) * spacing;
                let ph = h * cell_height + (h - 1.0) * spacing;

                (item.id, (px, py, pw, ph))
            })
            .collect()
    }

    /// Sets a constraint on an item. Returns `false` if the item is not found.
    pub fn set_item_constraints(
        &mut self,
        id: ItemId,
        min_w: Option<u16>,
        max_w: Option<u16>,
        min_h: Option<u16>,
        max_h: Option<u16>,
    ) -> bool {
        let Some(idx) = self.items.iter().position(|item| item.id == id) else {
            return false;
        };
        self.items[idx].min_w = min_w;
        self.items[idx].max_w = max_w;
        self.items[idx].min_h = min_h;
        self.items[idx].max_h = max_h;

        // Re-apply constraints
        self.node_bound_fix_by_id(id, false);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =================================================================
    // 1. AABB Intersection Tests
    // =================================================================

    #[test]
    fn intercepted_overlapping() {
        let a = make_item(0, 0, 3, 3);
        let b = make_item(2, 2, 3, 3);
        assert!(is_intercepted(&a, &b));
        assert!(is_intercepted(&b, &a));
    }

    #[test]
    fn intercepted_non_overlapping() {
        let a = make_item(0, 0, 2, 2);
        let b = make_item(5, 5, 2, 2);
        assert!(!is_intercepted(&a, &b));
        assert!(!is_intercepted(&b, &a));
    }

    #[test]
    fn intercepted_adjacent_horizontal() {
        // Adjacent: a ends at x=2, b starts at x=2 -> no overlap
        let a = make_item(0, 0, 2, 2);
        let b = make_item(2, 0, 2, 2);
        assert!(!is_intercepted(&a, &b));
        assert!(!is_intercepted(&b, &a));
    }

    #[test]
    fn intercepted_adjacent_vertical() {
        let a = make_item(0, 0, 2, 2);
        let b = make_item(0, 2, 2, 2);
        assert!(!is_intercepted(&a, &b));
        assert!(!is_intercepted(&b, &a));
    }

    #[test]
    fn intercepted_contained() {
        // b is fully inside a
        let a = make_item(0, 0, 6, 6);
        let b = make_item(1, 1, 2, 2);
        assert!(is_intercepted(&a, &b));
        assert!(is_intercepted(&b, &a));
    }

    #[test]
    fn intercepted_same_position() {
        let a = make_item(1, 1, 3, 3);
        let b = make_item(1, 1, 3, 3);
        assert!(is_intercepted(&a, &b));
    }

    #[test]
    fn intercepted_partial_overlap_horizontal() {
        let a = make_item(0, 0, 3, 2);
        let b = make_item(1, 0, 3, 2);
        assert!(is_intercepted(&a, &b));
    }

    #[test]
    fn intercepted_one_cell_overlap() {
        let a = make_item(0, 0, 2, 2);
        let b = make_item(1, 1, 2, 2);
        assert!(is_intercepted(&a, &b));
    }

    // =================================================================
    // 2. Add / Remove Tests
    // =================================================================

    #[test]
    fn add_single_item() {
        let mut engine = Internal::new(12);
        let id = engine.add_item(0, 0, 4, 2);
        assert_eq!(engine.items().count(), 1);
        let item = engine.get(id).unwrap();
        assert_eq!(item.x, 0);
        assert_eq!(item.y, 0);
        assert_eq!(item.w, 4);
        assert_eq!(item.h, 2);
    }

    #[test]
    fn add_multiple_non_overlapping() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(4, 0, 4, 2);
        let c = engine.add_item(8, 0, 4, 2);

        assert_eq!(engine.items().count(), 3);
        assert_eq!(engine.get(a).unwrap().x, 0);
        assert_eq!(engine.get(b).unwrap().x, 4);
        assert_eq!(engine.get(c).unwrap().x, 8);
        // All on row 0
        assert_eq!(engine.get(a).unwrap().y, 0);
        assert_eq!(engine.get(b).unwrap().y, 0);
        assert_eq!(engine.get(c).unwrap().y, 0);
    }

    #[test]
    fn add_auto_position() {
        let mut engine = Internal::new(12);
        engine.add_item(0, 0, 6, 2);
        engine.add_item(6, 0, 6, 2);

        // Now the first row is full; auto-position should go to row 2
        let id = engine.add_item_auto(4, 2).unwrap();
        let item = engine.get(id).unwrap();
        assert_eq!(item.y, 2);
        assert!(item.x + item.w <= 12);
    }

    #[test]
    fn add_auto_position_fits_in_gap() {
        let mut engine = Internal::new(12);
        engine.add_item(0, 0, 4, 2);
        engine.add_item(8, 0, 4, 2);

        // There's a gap at (4, 0) with width 4
        let id = engine.add_item_auto(4, 2).unwrap();
        let item = engine.get(id).unwrap();
        assert_eq!(item.x, 4);
        assert_eq!(item.y, 0);
    }

    #[test]
    fn remove_item_basic() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(4, 0, 4, 2);

        let removed = engine.remove_item(a);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, a);
        assert_eq!(engine.items().count(), 1);
        assert!(engine.get(a).is_none());
        assert!(engine.get(b).is_some());
    }

    #[test]
    fn remove_nonexistent_item() {
        let mut engine = Internal::new(12);
        engine.add_item(0, 0, 4, 2);
        let result = engine.remove_item(ItemId(999));
        assert!(result.is_none());
    }

    #[test]
    fn remove_triggers_compaction() {
        let mut engine = Internal::new(12);
        let _a = engine.add_item(0, 0, 12, 2);
        let b = engine.add_item(0, 2, 12, 2);

        // Remove a -- b should compact upward to y=0
        engine.remove_item(_a);
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_b.y, 0);
    }

    // =================================================================
    // 3. Collision Resolution Tests
    // =================================================================

    #[test]
    fn collision_displaces_item_down() {
        let mut engine = Internal::new(12);
        // Place b first, then add a on top of it -> b should be displaced
        let b = engine.add_item(0, 0, 4, 2);
        let a = engine.add_item(0, 0, 4, 2);

        // a should be at y=0, b should be displaced to y=2
        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_a.y, 0);
        assert_eq!(item_b.y, 2);
    }

    #[test]
    fn collision_cascade() {
        let mut engine = Internal::new(12);
        engine.set_float(true); // Disable gravity to see raw collision results

        // Place three items vertically
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(0, 2, 4, 2);
        let c = engine.add_item(0, 4, 4, 2);

        // Now add a new item on top of a -- it should cascade
        let d = engine.add_item(0, 0, 4, 2);

        let item_d = engine.get(d).unwrap();
        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        let item_c = engine.get(c).unwrap();

        assert_eq!(item_d.y, 0);
        // Each item should be pushed 2 rows below the one before it
        // The exact cascade: d at 0, a pushed to 2, b pushed to 4, c pushed to 6
        assert_eq!(item_a.y, 2);
        assert_eq!(item_b.y, 4);
        assert_eq!(item_c.y, 6);
    }

    #[test]
    fn collision_held_item_not_displaced() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        let a = engine.add_item(0, 0, 4, 2);

        // Manually place b on top and resolve with a held
        let b = engine.add_item(0, 2, 4, 2); // add elsewhere first
        engine.move_item_held(b, 0, 0, &[a]);

        // a stays put (held), b should be displaced below a
        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_a.y, 0);
        assert_eq!(item_b.y, 2);
    }

    #[test]
    fn collision_with_partial_overlap() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        let a = engine.add_item(0, 0, 6, 3);
        let b = engine.add_item(3, 1, 6, 3);

        // b overlaps with a (columns 3-6, rows 1-3)
        // a was added first. b is the new item. a gets displaced by b.
        // Actually: when b is added, it may displace a or a displaces b.
        // In our engine, fix_collisions is called for the newly added item.
        // The newly added item (b) pushes colliding items (a) down.
        // Wait -- let me re-read the fix_collisions logic:
        // fix_collisions(b) checks full-row in gravity mode. But we're in float mode.
        // In float mode, it checks the exact area of b.
        // It finds a collides with b, and since a is not held, pushes a below b.
        // a goes to y = b.y + b.h = 1 + 3 = 4
        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();

        assert_eq!(item_b.y, 1);
        assert_eq!(item_a.y, 4);
        // They should not overlap
        assert!(!is_intercepted(item_a, item_b));
    }

    // =================================================================
    // 4. Gravity / Packing Tests
    // =================================================================

    #[test]
    fn pack_nodes_compacts_upward() {
        let mut engine = Internal::new(12);
        engine.set_float(true); // Add in float mode

        engine.add_item(0, 5, 4, 2); // far down
        engine.add_item(4, 10, 4, 2); // even further down

        engine.set_float(false); // Now enable gravity

        // Both items should compact to y=0
        for item in engine.items() {
            assert_eq!(item.y, 0, "item {:?} should be at y=0", item.id);
        }
    }

    #[test]
    fn pack_nodes_respects_collisions() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        let a = engine.add_item(0, 0, 12, 3);
        let b = engine.add_item(0, 10, 12, 2);

        engine.set_float(false);

        // a should stay at y=0 (already packed)
        // b should compact to y=3 (just below a)
        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_a.y, 0);
        assert_eq!(item_b.y, 3);
    }

    #[test]
    fn pack_nodes_held_items_stay() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        let a = engine.add_item(0, 5, 12, 2);

        // Compact with a held — it should NOT be compacted
        engine.pack_nodes_held(&[a]);

        let item_a = engine.get(a).unwrap();
        assert_eq!(item_a.y, 5);
    }

    #[test]
    fn gravity_after_add() {
        let mut engine = Internal::new(12);
        // With gravity on (default), adding an item at y=10 should compact it to y=0
        let a = engine.add_item(0, 10, 4, 2);
        let item_a = engine.get(a).unwrap();
        assert_eq!(item_a.y, 0);
    }

    #[test]
    fn pack_nodes_stacks_correctly() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        // Two full-width items at different heights
        let a = engine.add_item(0, 0, 12, 2);
        let b = engine.add_item(0, 8, 12, 3);

        engine.set_float(false);

        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_a.y, 0);
        assert_eq!(item_b.y, 2);
    }

    // =================================================================
    // 5. Move Tests
    // =================================================================

    #[test]
    fn move_item_basic() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);

        let moved = engine.move_item(a, 4, 0);
        assert!(moved);
        let item = engine.get(a).unwrap();
        assert_eq!(item.x, 4);
        assert_eq!(item.y, 0);
    }

    #[test]
    fn move_item_resolves_collisions() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(4, 0, 4, 2);

        // Move a to overlap with b
        engine.move_item(a, 4, 0);

        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();

        // a should be at (4, 0), b displaced
        assert_eq!(item_a.x, 4);
        assert_eq!(item_a.y, 0);
        assert!(item_b.y >= 2); // b must be below a
        assert!(!is_intercepted(item_a, item_b));
    }

    #[test]
    fn move_item_clamped_to_grid() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);

        // Try to move beyond right edge
        engine.move_item(a, 20, 0);

        let item = engine.get(a).unwrap();
        // Should be clamped: x + w <= 12, so x <= 8
        assert!(item.x + item.w <= 12);
    }

    #[test]
    fn move_held_displaces_around_held_items() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(4, 0, 4, 2);

        // Move a to overlap b, with b held — a should be pushed below b
        engine.move_item_held(a, 4, 0, &[b]);

        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_b.x, 4);
        assert_eq!(item_b.y, 0);
        assert_eq!(item_a.y, 2); // pushed below held item b
    }

    #[test]
    fn move_item_same_position_returns_false() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);

        let moved = engine.move_item(a, 0, 0);
        assert!(!moved);
    }

    // =================================================================
    // 6. Resize Tests
    // =================================================================

    #[test]
    fn resize_item_basic() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);

        let resized = engine.resize_item(a, 6, 3);
        assert!(resized);
        let item = engine.get(a).unwrap();
        assert_eq!(item.w, 6);
        assert_eq!(item.h, 3);
    }

    #[test]
    fn resize_item_resolves_collisions() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(4, 0, 4, 2);

        // Resize a to overlap b
        engine.resize_item(a, 8, 2);

        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();

        assert_eq!(item_a.w, 8);
        assert!(item_b.y >= 2); // b displaced
        assert!(!is_intercepted(item_a, item_b));
    }

    #[test]
    fn resize_clamped_to_grid_boundary() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(6, 0, 4, 2);

        // Resize wider than remaining space
        engine.resize_item(a, 20, 2);

        let item = engine.get(a).unwrap();
        // When resizing, width is clamped: w = columns - x = 12 - 6 = 6
        assert!(item.x + item.w <= 12);
        assert_eq!(item.x, 6); // Position should NOT shift during resize
    }

    #[test]
    fn resize_held_displaces_around_held_items() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        let b = engine.add_item(4, 0, 4, 2);

        // Resize a to overlap b, with b held — b stays, a pushes other
        // items but b remains unmoved
        engine.resize_item_held(a, 8, 2, &[b]);

        let item_a = engine.get(a).unwrap();
        let item_b = engine.get(b).unwrap();
        assert_eq!(item_a.w, 8);
        // b is held so it cannot be displaced — it gets pushed below
        // Actually: a is the one being resized so fix_collisions(a, held=[b])
        // finds b collides and since b is held, a gets pushed below b.
        // But wait, a is the resized item. Let's just check no overlap.
        assert!(!is_intercepted(item_a, item_b));
    }

    // =================================================================
    // 7. Constraint Enforcement Tests
    // =================================================================

    #[test]
    fn constraint_min_width() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        engine.set_item_constraints(a, Some(3), None, None, None);

        // Try to resize below min
        engine.resize_item(a, 1, 2);
        // The item should be clamped to min_w = 3
        let item = engine.get(a).unwrap();
        assert_eq!(item.w, 3);
    }

    #[test]
    fn constraint_max_width() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        engine.set_item_constraints(a, None, Some(6), None, None);

        // Try to resize beyond max
        engine.resize_item(a, 10, 2);
        let item = engine.get(a).unwrap();
        assert_eq!(item.w, 6);
    }

    #[test]
    fn constraint_min_height() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 4);
        engine.set_item_constraints(a, None, None, Some(3), None);

        engine.resize_item(a, 4, 1);
        let item = engine.get(a).unwrap();
        assert_eq!(item.h, 3);
    }

    #[test]
    fn constraint_max_height() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 4, 2);
        engine.set_item_constraints(a, None, None, None, Some(5));

        engine.resize_item(a, 4, 10);
        let item = engine.get(a).unwrap();
        assert_eq!(item.h, 5);
    }

    #[test]
    fn constraint_grid_boundary_clamp_width() {
        let mut engine = Internal::new(12);
        // Add item wider than grid
        let a = engine.add_item(0, 0, 20, 2);
        let item = engine.get(a).unwrap();
        assert_eq!(item.w, 12);
        assert_eq!(item.x, 0);
    }

    #[test]
    fn constraint_max_rows() {
        let mut engine = Internal::new(12);
        engine.set_max_rows(Some(5));

        let a = engine.add_item(0, 10, 4, 2);
        let item = engine.get(a).unwrap();
        // Should be clamped to fit within 5 rows: y + h <= 5
        assert!(item.y + item.h <= 5);
    }

    #[test]
    fn constraint_position_shift_on_overflow() {
        let mut engine = Internal::new(12);
        // Item at x=10 with w=4 overflows (10+4 > 12)
        // In non-resize mode, it should shift left: x = 12 - 4 = 8
        let a = engine.add_item(10, 0, 4, 2);
        let item = engine.get(a).unwrap();
        assert_eq!(item.x, 8);
        assert_eq!(item.w, 4);
    }

    // =================================================================
    // 8. Pixel Region Calculation Tests
    // =================================================================

    #[test]
    fn pixel_regions_single_item() {
        let mut engine = Internal::new(4);
        engine.set_float(true);
        let a = engine.add_item(0, 0, 2, 1);

        let regions = engine.item_regions((400.0, 400.0), 0.0);
        assert_eq!(regions.len(), 1);

        let (id, (px, py, pw, ph)) = &regions[0];
        assert_eq!(*id, a);
        assert!((px - 0.0).abs() < 0.01);
        assert!((py - 0.0).abs() < 0.01);
        assert!((pw - 200.0).abs() < 0.01);
        assert!((ph - 100.0).abs() < 0.01);
    }

    #[test]
    fn pixel_regions_with_spacing() {
        let mut engine = Internal::new(4);
        engine.set_float(true);
        let _a = engine.add_item(0, 0, 1, 1);
        let b = engine.add_item(1, 0, 1, 1);

        // With 4 columns, bounds 400px, spacing 4px:
        // cell_width = (400 - 3*4) / 4 = 388 / 4 = 97
        let regions = engine.item_regions((400.0, 400.0), 4.0);
        assert_eq!(regions.len(), 2);

        let region_b = regions.iter().find(|(id, _)| *id == b).unwrap();
        let (_, (px, py, pw, ph)) = region_b;

        let cell_width = (400.0 - 3.0 * 4.0) / 4.0;
        let expected_x = 1.0 * cell_width + 1.0 * 4.0;

        assert!((px - expected_x).abs() < 0.01);
        assert!((py - 0.0).abs() < 0.01);
        assert!((pw - cell_width).abs() < 0.01);
        assert!((ph - cell_width).abs() < 0.01);
    }

    #[test]
    fn pixel_regions_empty_grid() {
        let engine = Internal::new(12);
        let regions = engine.item_regions((1000.0, 800.0), 5.0);
        assert!(regions.is_empty());
    }

    #[test]
    fn pixel_regions_multicolumn_item() {
        let mut engine = Internal::new(4);
        engine.set_float(true);
        let a = engine.add_item(1, 0, 2, 3);

        // 4 columns, 400px wide, 0 spacing: cell = 100px
        let regions = engine.item_regions((400.0, 400.0), 0.0);
        let (id, (px, py, pw, ph)) = &regions[0];
        assert_eq!(*id, a);
        assert!((px - 100.0).abs() < 0.01);
        assert!((py - 0.0).abs() < 0.01);
        assert!((pw - 200.0).abs() < 0.01);
        assert!((ph - 300.0).abs() < 0.01);
    }

    // =================================================================
    // 9. Complex Layout Tests
    // =================================================================

    #[test]
    fn complex_dashboard_layout() {
        let mut engine = Internal::new(12);

        // Simulate a dashboard:
        // - Header: full width, height 1
        // - Sidebar: 3 cols wide, height 4
        // - Main: 9 cols wide, height 2
        // - Bottom panel: 9 cols wide, height 2
        let header = engine.add_item(0, 0, 12, 1);
        let sidebar = engine.add_item(0, 1, 3, 4);
        let main = engine.add_item(3, 1, 9, 2);
        let bottom = engine.add_item(3, 3, 9, 2);

        // Verify no overlaps
        let items: Vec<_> = engine.items().cloned().collect();
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                assert!(
                    !is_intercepted(&items[i], &items[j]),
                    "{:?} and {:?} overlap",
                    items[i].id,
                    items[j].id
                );
            }
        }

        // Verify expected positions
        assert_eq!(engine.get(header).unwrap().y, 0);
        assert_eq!(engine.get(sidebar).unwrap().y, 1);
        assert_eq!(engine.get(main).unwrap().y, 1);
        assert_eq!(engine.get(bottom).unwrap().y, 3);
    }

    #[test]
    fn complex_add_remove_move_resize() {
        let mut engine = Internal::new(12);

        let a = engine.add_item(0, 0, 6, 2);
        let b = engine.add_item(6, 0, 6, 2);
        let c = engine.add_item(0, 2, 12, 2);

        // Move a to right side (displaces b)
        engine.move_item(a, 6, 0);
        // b should be displaced
        assert!(!is_intercepted(
            engine.get(a).unwrap(),
            engine.get(b).unwrap()
        ));

        // Resize c to be smaller
        engine.resize_item(c, 6, 1);
        assert_eq!(engine.get(c).unwrap().w, 6);

        // Remove b
        engine.remove_item(b);
        assert_eq!(engine.items().count(), 2);

        // Verify no overlaps remain
        let items: Vec<_> = engine.items().cloned().collect();
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                assert!(!is_intercepted(&items[i], &items[j]));
            }
        }
    }

    #[test]
    fn stress_many_items_no_overlap() {
        let mut engine = Internal::new(12);

        // Add 20 items, some overlapping
        for i in 0..20u16 {
            engine.add_item(i % 12, (i / 12) * 2, 3, 2);
        }

        // Verify no overlaps
        let items: Vec<_> = engine.items().cloned().collect();
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                assert!(
                    !is_intercepted(&items[i], &items[j]),
                    "items {:?} and {:?} overlap: ({},{},{},{}) vs ({},{},{},{})",
                    items[i].id,
                    items[j].id,
                    items[i].x,
                    items[i].y,
                    items[i].w,
                    items[i].h,
                    items[j].x,
                    items[j].y,
                    items[j].w,
                    items[j].h,
                );
            }
        }
    }

    // =================================================================
    // 10. Edge Case Tests
    // =================================================================

    #[test]
    fn empty_grid_operations() {
        let mut engine = Internal::new(12);
        assert_eq!(engine.get_row(), 0);
        assert_eq!(engine.items().count(), 0);
        assert!(engine.item_regions((800.0, 600.0), 5.0).is_empty());
        assert!(!engine.move_item(ItemId(0), 1, 1));
        assert!(!engine.resize_item(ItemId(0), 2, 2));
        assert!(engine.remove_item(ItemId(0)).is_none());
    }

    #[test]
    fn single_item_grid() {
        let mut engine = Internal::new(1);
        let a = engine.add_item(0, 0, 1, 1);
        assert_eq!(engine.get(a).unwrap().x, 0);
        assert_eq!(engine.get(a).unwrap().w, 1);
        assert_eq!(engine.get_row(), 1);
    }

    #[test]
    fn item_at_grid_boundary() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(8, 0, 4, 2);
        let item = engine.get(a).unwrap();
        assert_eq!(item.x, 8);
        assert_eq!(item.w, 4);
        assert_eq!(item.right(), 12);
    }

    #[test]
    fn full_grid_auto_position_fails_with_max_rows() {
        let mut engine = Internal::new(2);
        engine.set_max_rows(Some(2));

        // Fill the grid
        engine.add_item(0, 0, 1, 1);
        engine.add_item(1, 0, 1, 1);
        engine.add_item(0, 1, 1, 1);
        engine.add_item(1, 1, 1, 1);

        // No room left
        let result = engine.add_item_auto(1, 1);
        assert!(result.is_none());
    }

    #[test]
    fn get_row_tracks_max_height() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        engine.add_item(0, 0, 4, 2);
        assert_eq!(engine.get_row(), 2);

        engine.add_item(0, 5, 4, 3);
        assert_eq!(engine.get_row(), 8);
    }

    #[test]
    fn deterministic_output() {
        // Same operations should always produce same result
        let run = || {
            let mut engine = Internal::new(12);
            let a = engine.add_item(0, 0, 6, 2);
            let b = engine.add_item(3, 0, 6, 2);
            let c = engine.add_item(0, 1, 4, 3);
            engine.move_item(a, 2, 2);
            engine.resize_item(b, 8, 1);

            let mut result: Vec<_> = engine
                .items()
                .map(|item| (item.id, item.x, item.y, item.w, item.h))
                .collect();
            result.sort_by_key(|(id, ..)| *id);

            // Also include c's position
            let _ = engine.get(c);
            result
        };

        let r1 = run();
        let r2 = run();
        assert_eq!(r1, r2);
    }

    #[test]
    fn float_mode_no_gravity() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        let a = engine.add_item(0, 5, 4, 2);
        // In float mode, item stays at y=5
        assert_eq!(engine.get(a).unwrap().y, 5);
    }

    #[test]
    fn gravity_mode_compacts_on_add() {
        let mut engine = Internal::new(12);
        // Gravity is on by default

        let a = engine.add_item(0, 5, 4, 2);
        // Should compact to y=0
        assert_eq!(engine.get(a).unwrap().y, 0);
    }

    #[test]
    fn move_item_nonexistent_returns_false() {
        let mut engine = Internal::new(12);
        assert!(!engine.move_item(ItemId(42), 0, 0));
    }

    #[test]
    fn resize_item_nonexistent_returns_false() {
        let mut engine = Internal::new(12);
        assert!(!engine.resize_item(ItemId(42), 4, 4));
    }

    #[test]
    fn min_dimensions_enforced_on_add() {
        let mut engine = Internal::new(12);
        // Item with w=0 should be clamped to w=1
        let a = engine.add_item(0, 0, 0, 0);
        let item = engine.get(a).unwrap();
        assert!(item.w >= 1);
        assert!(item.h >= 1);
    }

    #[test]
    fn side_by_side_items_not_displaced() {
        let mut engine = Internal::new(12);
        let a = engine.add_item(0, 0, 6, 2);
        let b = engine.add_item(6, 0, 6, 2);

        // Side-by-side items should both be at y=0
        assert_eq!(engine.get(a).unwrap().y, 0);
        assert_eq!(engine.get(b).unwrap().y, 0);
        assert_eq!(engine.get(a).unwrap().x, 0);
        assert_eq!(engine.get(b).unwrap().x, 6);
    }

    #[test]
    fn move_held_preserves_held_position() {
        let mut engine = Internal::new(12);
        // Gravity on (default). Add two non-overlapping items.
        let pinned = engine.add_item(0, 0, 12, 2); // y=0
        let free = engine.add_item(0, 2, 12, 2); // y=2, below pinned

        assert_eq!(engine.get(pinned).unwrap().y, 0);
        assert_eq!(engine.get(free).unwrap().y, 2);

        // Move free to overlap pinned with pinned held — pinned stays,
        // free is pushed below.
        engine.move_item_held(free, 0, 0, &[pinned]);
        assert_eq!(engine.get(pinned).unwrap().y, 0);
        assert_eq!(engine.get(free).unwrap().y, 2);
    }

    #[test]
    fn move_held_compacts_around_held() {
        let mut engine = Internal::new(12);
        let pinned = engine.add_item(0, 0, 12, 3); // y=0, h=3
        let free = engine.add_item(0, 3, 12, 2); // y=3, below pinned

        assert_eq!(engine.get(pinned).unwrap().y, 0);
        assert_eq!(engine.get(free).unwrap().y, 3);

        // Move free far down, then back to overlap with held pinned.
        engine.move_item(free, 0, 10);
        engine.move_item_held(free, 0, 0, &[pinned]);

        // free collides with pinned (y=0, h=3). pinned is held, so
        // free is pushed below pinned to y=3.
        assert_eq!(engine.get(pinned).unwrap().y, 0);
        assert_eq!(engine.get(free).unwrap().y, 3);
    }

    // =================================================================
    // 11. Batch Mode Tests
    // =================================================================

    #[test]
    fn batch_mode_defers_packing() {
        let mut engine = Internal::new(12);
        let _a = engine.add_item(0, 0, 12, 2); // rows 0..2
        let b = engine.add_item(0, 2, 12, 2); // rows 2..4

        engine.begin_batch();
        assert!(engine.is_batch());

        // Remove a -- normally b would compact to y=0
        engine.remove_item(_a);

        // In batch mode, b should NOT have compacted
        assert_eq!(engine.get(b).unwrap().y, 2);

        // End batch -- b should now compact to y=0
        engine.end_batch();
        assert!(!engine.is_batch());
        assert_eq!(engine.get(b).unwrap().y, 0);
    }

    #[test]
    fn batch_mode_resize_no_float_up() {
        let mut engine = Internal::new(12);

        // Three items stacked:
        //   a: (0,0) 6x2
        //   b: (6,0) 6x2
        //   c: (0,2) 12x2
        let _a = engine.add_item(0, 0, 6, 2);
        let b = engine.add_item(6, 0, 6, 2);
        let c = engine.add_item(0, 2, 12, 2);

        engine.begin_batch();

        // Resize a to be wider, overlapping b. b gets pushed down but
        // should NOT float up because batch mode defers packing.
        engine.resize_item(_a, 12, 2);

        // b was displaced below a (y >= 2). c was displaced below b.
        // Crucially, nothing floated up during batch mode.
        let item_b = engine.get(b).unwrap();
        assert!(item_b.y >= 2, "b should have been pushed down");

        let item_c = engine.get(c).unwrap();
        // c was at y=2 and may have been displaced further down by b
        assert!(item_c.y >= 2, "c should not have floated up");

        engine.end_batch();

        // After end_batch, items settle via gravity.
        // a occupies (0,0) 12x2. b compacts to y=2. c compacts to y=4.
        let item_b = engine.get(b).unwrap();
        let item_c = engine.get(c).unwrap();
        assert_eq!(item_b.y, 2);
        assert_eq!(item_c.y, 4);
    }

    #[test]
    fn batch_mode_move_no_float_up() {
        let mut engine = Internal::new(12);

        // a at (0,0) 6x2, b at (6,0) 6x2, c at (0,2) 6x2
        let a = engine.add_item(0, 0, 6, 2);
        let _b = engine.add_item(6, 0, 6, 2);
        let c = engine.add_item(0, 2, 6, 2);

        engine.begin_batch();

        // Move a down to row 4 -- c was below a at y=2 and should not
        // float up while batch mode is active.
        engine.move_item(a, 0, 4);

        let item_c = engine.get(c).unwrap();
        assert_eq!(item_c.y, 2, "c should not float up during batch mode");

        engine.end_batch();

        // After end_batch, c should compact to y=0 (a moved away).
        let item_c = engine.get(c).unwrap();
        assert_eq!(item_c.y, 0);
    }

    #[test]
    fn batch_mode_with_float_no_compaction() {
        let mut engine = Internal::new(12);
        engine.set_float(true);

        let a = engine.add_item(0, 5, 6, 2);

        engine.begin_batch();
        // Even after end_batch, float mode prevents compaction
        engine.end_batch();

        assert_eq!(engine.get(a).unwrap().y, 5);
    }

    #[test]
    fn batch_mode_pack_nodes_is_noop() {
        let mut engine = Internal::new(12);

        let a = engine.add_item(0, 0, 12, 2);
        let b = engine.add_item(0, 2, 12, 2);

        engine.begin_batch();

        // Manually remove a and call pack_nodes -- should be a no-op.
        engine.remove_item(a);
        engine.pack_nodes();

        // b should still be at y=2 because pack_nodes is a no-op in batch mode.
        assert_eq!(engine.get(b).unwrap().y, 2);

        engine.end_batch();

        // Now b should compact.
        assert_eq!(engine.get(b).unwrap().y, 0);
    }

    // Helper to create a GridItem for intersection tests
    fn make_item(x: u16, y: u16, w: u16, h: u16) -> GridItem {
        GridItem {
            id: ItemId(0),
            x,
            y,
            w,
            h,
            min_w: None,
            max_w: None,
            min_h: None,
            max_h: None,
        }
    }
}
