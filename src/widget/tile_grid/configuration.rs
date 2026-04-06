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
///     .with_item(0, 0, 4, 2, "sidebar")
///     .with_item(4, 0, 8, 2, "main");
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

    /// Appends an item at the given position and size with the provided
    /// state.
    ///
    /// This is a shorthand for `.push(Item::new(x, y, w, h, state))`.
    #[must_use]
    pub fn with_item(self, x: u16, y: u16, w: u16, h: u16, state: T) -> Self {
        self.push(Item::new(x, y, w, h, state))
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
}

impl<T> Item<T> {
    /// Creates a new [`Item`] at the given position and size.
    ///
    /// All constraints default to `None`.
    #[must_use]
    pub fn new(x: u16, y: u16, w: u16, h: u16, state: T) -> Self {
        Self {
            x,
            y,
            w,
            h,
            min_w: None,
            max_w: None,
            min_h: None,
            max_h: None,
            state,
        }
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
