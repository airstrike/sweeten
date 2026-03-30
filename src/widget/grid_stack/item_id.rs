//! Unique identifier for grid items.

use std::fmt;

/// The unique identifier of an item in a [`GridEngine`].
///
/// This is a simple newtype wrapper around `usize`, similar to
/// [`iced`'s `Pane`](https://docs.iced.rs/iced/widget/pane_grid/struct.Pane.html).
/// IDs are assigned monotonically by the engine and are never reused.
///
/// [`GridEngine`]: super::GridEngine
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub(crate) usize);

impl fmt::Debug for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ItemId({})", self.0)
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
