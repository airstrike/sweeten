//! Declarative configuration for building a [`TileGrid`] layout.
//!
//! A [`Configuration`] describes the initial arrangement of a tile grid:
//! the number of columns, optional constraints, and the items with their
//! positions and user data. Pass it to [`State::with_configuration`] to
//! construct a ready-to-use [`State`].
//!
//! [`TileGrid`]: super::TileGrid
//! [`State`]: super::State
//! [`State::with_configuration`]: super::State::with_configuration

use super::rect::Rect;

/// The arrangement of a [`TileGrid`].
///
/// Describes a flat grid layout with a fixed number of columns and a
/// list of positioned items. This is the tile-grid analogue of
/// [`pane_grid::Configuration`], adapted from a recursive split tree
/// to a flat coordinate list.
///
/// # Example
///
/// ```
/// use sweeten::widget::tile_grid::{Configuration, configuration::Item};
///
/// let config: Configuration<&str> = Configuration::new(12)
///     .with_item([0, 0, 4, 2], "sidebar")
///     .with_item([4, 0, 8, 2], "main");
/// ```
///
/// [`TileGrid`]: super::TileGrid
/// [`pane_grid::Configuration`]: https://docs.iced.rs/iced/widget/pane_grid/enum.Configuration.html
#[derive(Debug, Clone)]
pub struct Configuration<T> {
    /// Number of columns in the grid.
    pub columns: u16,
    /// Maximum number of rows (`None` = unlimited).
    pub max_rows: Option<u16>,
    /// Float mode: when `false` (default), items compact upward under
    /// gravity.
    pub float: bool,
    /// The items to place on the grid.
    pub items: Vec<Item<T>>,
}

impl<T> Configuration<T> {
    /// Creates a new [`Configuration`] with the given number of columns.
    ///
    /// Defaults to no row limit, gravity enabled (`float = false`), and
    /// an empty item list.
    #[must_use]
    pub fn new(columns: u16) -> Self {
        Self {
            columns,
            max_rows: None,
            float: false,
            items: Vec::new(),
        }
    }

    /// Sets the maximum number of rows.
    #[must_use]
    pub fn max_rows(mut self, max_rows: u16) -> Self {
        self.max_rows = Some(max_rows);
        self
    }

    /// Enables or disables float mode.
    #[must_use]
    pub fn float(mut self, float: bool) -> Self {
        self.float = float;
        self
    }

    /// Appends a pre-built [`Item`] to the configuration.
    #[must_use]
    pub fn push(mut self, item: Item<T>) -> Self {
        self.items.push(item);
        self
    }

    /// Appends an item at the given grid region with the provided state.
    ///
    /// This is a shorthand for `.push(Item::new(rect, state))`.
    #[must_use]
    pub fn with_item(self, rect: impl Into<Rect>, state: T) -> Self {
        self.push(Item::new(rect, state))
    }
}

/// A single item in a [`Configuration`].
///
/// Describes an item's grid position, size, optional constraints, and
/// user data. The item does not yet have an [`ItemId`] — one will be
/// assigned when the configuration is loaded into a [`State`].
///
/// [`ItemId`]: super::ItemId
/// [`State`]: super::State
#[derive(Debug, Clone)]
pub struct Item<T> {
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
    /// User data associated with the item.
    pub state: T,
    /// Forces this item to be a *container* (a "group") even with no children
    /// yet. An item with children is a container regardless. The child grid's
    /// column count is the item's own width.
    pub group: bool,
    /// Child items. A non-empty list (or [`group`](Self::group)) makes this
    /// item a *container* (a "group") whose body hosts a nested grid.
    pub children: Vec<Item<T>>,
}

impl<T> Item<T> {
    /// Creates a new [`Item`] at the given grid region.
    ///
    /// All constraints default to `None` and the item is a leaf (no
    /// children).
    #[must_use]
    pub fn new(rect: impl Into<Rect>, state: T) -> Self {
        let rect = rect.into();
        Self {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            min_w: None,
            max_w: None,
            min_h: None,
            max_h: None,
            state,
            group: false,
            children: Vec::new(),
        }
    }

    /// Returns `true` if this item is a container (has children or is marked
    /// as a [`group`](Self::group)).
    #[must_use]
    pub fn is_group(&self) -> bool {
        self.group || !self.children.is_empty()
    }

    /// Marks this item as a container (a "group") even when it has no children
    /// yet. Its child grid takes the item's own width as its column count.
    #[must_use]
    pub fn group(mut self) -> Self {
        self.group = true;
        self
    }

    /// Appends a child item, marking this item as a container.
    #[must_use]
    pub fn child(mut self, child: Item<T>) -> Self {
        self.children.push(child);
        self
    }

    /// Appends a child item at the given grid region.
    #[must_use]
    pub fn with_child(self, rect: impl Into<Rect>, state: T) -> Self {
        self.child(Item::new(rect, state))
    }

    /// Sets the minimum width constraint.
    #[must_use]
    pub fn min_w(mut self, min_w: u16) -> Self {
        self.min_w = Some(min_w);
        self
    }

    /// Sets the maximum width constraint.
    #[must_use]
    pub fn max_w(mut self, max_w: u16) -> Self {
        self.max_w = Some(max_w);
        self
    }

    /// Sets the minimum height constraint.
    #[must_use]
    pub fn min_h(mut self, min_h: u16) -> Self {
        self.min_h = Some(min_h);
        self
    }

    /// Sets the maximum height constraint.
    #[must_use]
    pub fn max_h(mut self, max_h: u16) -> Self {
        self.max_h = Some(max_h);
        self
    }
}
