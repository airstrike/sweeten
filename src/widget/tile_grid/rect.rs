//! A rectangular region in integer grid coordinates.

use crate::core::Rectangle;

/// A rectangular region of grid cells: a column/row origin (`x`, `y`) and a
/// size (`w`, `h`) measured in columns and rows.
///
/// This is the grid-space counterpart to `iced`'s pixel
/// [`Rectangle`](crate::core::Rectangle). The tile-grid API takes
/// `impl Into<Rect>`, so a position can be given as a [`Rect`], a `[u16; 4]`
/// (`[x, y, w, h]`), a `(u16, u16, u16, u16)` tuple, or an `iced`
/// `Rectangle<u16>` — whichever reads best at the call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Column of the left edge (0-based, from the left).
    pub x: u16,
    /// Row of the top edge (0-based, from the top).
    pub y: u16,
    /// Width in columns.
    pub w: u16,
    /// Height in rows.
    pub h: u16,
}

impl Rect {
    /// Creates a region at `(x, y)` spanning `w` columns and `h` rows.
    #[must_use]
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }
}

impl From<[u16; 4]> for Rect {
    /// `[x, y, w, h]`.
    fn from([x, y, w, h]: [u16; 4]) -> Self {
        Self { x, y, w, h }
    }
}

impl From<(u16, u16, u16, u16)> for Rect {
    /// `(x, y, w, h)`.
    fn from((x, y, w, h): (u16, u16, u16, u16)) -> Self {
        Self { x, y, w, h }
    }
}

impl From<Rectangle<u16>> for Rect {
    /// `iced`'s `width`/`height` become `w`/`h`.
    fn from(r: Rectangle<u16>) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.width,
            h: r.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_agree() {
        let want = Rect::new(1, 2, 3, 4);
        assert_eq!(Rect::from([1, 2, 3, 4]), want);
        assert_eq!(Rect::from((1, 2, 3, 4)), want);
        assert_eq!(
            Rect::from(Rectangle {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            }),
            want
        );
    }
}
