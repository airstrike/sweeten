//! Pure drop-target resolution.
//!
//! Maps a cursor's vertical position to where a dragged node lands among one
//! grid level's children. The widget drives the geometry (rendered rects +
//! committed rows) and the recursion into containers; this module is just the
//! decision, kept free of `iced` so it can be unit-tested in isolation.
//!
//! Each child claims a vertical span of the canvas. For a **container** that
//! span is split three ways:
//!
//! ```text
//!  top              a container's rendered span               bottom
//!  ├──────────┬───────────────────────────────────┬──────────┤
//!  │  before  │                into                │  after*  │
//!  └──────────┴───────────────────────────────────┴──────────┘
//!   edge band              interior                  edge band
//! ```
//!
//! - **before** (top edge band) → insert into the *parent* grid, at this
//!   child's row (pushing it down).
//! - **into** (interior) → descend into this container and resolve again.
//! - **after** (bottom edge band) → insert into the *parent* grid, after this
//!   child. Only the *last* child has an after-band; for the others the
//!   interior runs to the bottom edge, so the next child's before-band (right
//!   below it, since stacked groups share no gap) is the sole "between" zone.
//!
//! A **leaf** has no interior to drop into, so its span splits at the midpoint:
//! above → before it, below → after it.
//!
//! So for `Pulse` over `Trends` (rendered `[0,100]` and `[100,200]`, band 20):
//! `[0,20)` before Pulse · `[20,100)` into Pulse · `[100,120)` before Trends
//! (between) · `[120,180)` into Trends · `[180,…)` after Trends (below).

use super::item_id::ItemId;

/// One child of the grid being resolved, in committed-row order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DropChild {
    /// The child's id.
    pub id: ItemId,
    /// The child's committed row in this grid.
    pub row: u16,
    /// The child's committed height, in rows. The row *past* this child is
    /// `row + h`, which is where an "after the last child" drop lands — a
    /// tall tile at row 0, height 2, spans rows 0–1, so below it is row 2.
    pub h: u16,
    /// Rendered top edge, in the same vertical space as the cursor.
    pub top: f32,
    /// Rendered bottom edge.
    pub bottom: f32,
    /// Whether the child is a container (has an interior to drop *into*).
    pub is_group: bool,
}

/// Where a drop resolves within one grid level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DropSlot {
    /// Insert into *this* grid at the given committed row.
    Row(u16),
    /// Descend into this container and resolve again among its children.
    Into(ItemId),
}

/// Resolves a vertical cursor position to a [`DropSlot`] among `children`.
///
/// `children` must already be filtered to those sharing the dragged node's
/// target column and sorted by row (top to bottom). `band_fraction` is the
/// edge band as a fraction of each child's *own* rendered height, so a tall
/// group and a short one frame their edges proportionally.
pub(crate) fn resolve_drop_y(
    children: &[DropChild],
    cursor_y: f32,
    band_fraction: f32,
) -> DropSlot {
    let last = children.len().saturating_sub(1);

    for (i, child) in children.iter().enumerate() {
        let band = band_fraction * (child.bottom - child.top);
        if child.is_group {
            // Top edge band → before this container.
            if cursor_y < child.top + band {
                return DropSlot::Row(child.row);
            }
            // Interior → into it. The last child reserves a bottom band for
            // "after"; the others run to their bottom edge (the next child's
            // top band is the between-zone).
            let interior_bottom = if i == last {
                child.bottom - band
            } else {
                child.bottom
            };
            if cursor_y < interior_bottom {
                return DropSlot::Into(child.id);
            }
            // Below the interior: fall through to the next child, or — if this
            // is the last — to the after-the-last result below.
        } else {
            // A leaf splits at its midpoint: above → before it; below → fall
            // through (after it / before the next child).
            if cursor_y < (child.top + child.bottom) / 2.0 {
                return DropSlot::Row(child.row);
            }
        }
    }

    // Past every child: insert below the last one. A child of height `h` at
    // `row` occupies `row..row + h`, so the first free row beneath it is
    // `row + h` — using `row + 1` would land inside a multi-row tile and
    // collide instead of appending a fresh row. Row 0 if the grid is empty.
    DropSlot::Row(children.last().map_or(0, |c| c.row + c.h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(id: u16, row: u16, top: f32, bottom: f32) -> DropChild {
        DropChild {
            id: ItemId(id as usize),
            row,
            h: 1,
            top,
            bottom,
            is_group: true,
        }
    }

    /// The canonical behavior: a `Pulse` group stacked directly on `Trends`,
    /// each rendered 100px tall with no gap, edge band 20px. Sweeping the
    /// cursor from the top of the canvas to the bottom must pass through
    /// exactly five zones in order: above Pulse, into Pulse, between, into
    /// Trends, below Trends.
    #[test]
    fn five_zones_above_within_between_within_below() {
        let pulse = group(0, 0, 0.0, 100.0);
        let trends = group(5, 1, 100.0, 200.0);
        let children = [pulse, trends];
        let band = 0.2; // fraction of each child's height
        let at = |y: f32| resolve_drop_y(&children, y, band);

        // above Pulse → insert at row 0 (before Pulse)
        assert_eq!(at(0.0), DropSlot::Row(0));
        assert_eq!(at(19.9), DropSlot::Row(0));

        // within Pulse → into it
        assert_eq!(at(20.0), DropSlot::Into(pulse.id));
        assert_eq!(at(99.9), DropSlot::Into(pulse.id));

        // between Pulse and Trends → insert at row 1 (before Trends)
        assert_eq!(at(100.0), DropSlot::Row(1));
        assert_eq!(at(119.9), DropSlot::Row(1));

        // within Trends → into it
        assert_eq!(at(120.0), DropSlot::Into(trends.id));
        assert_eq!(at(179.9), DropSlot::Into(trends.id));

        // below Trends → insert at row 2 (after Trends)
        assert_eq!(at(180.0), DropSlot::Row(2));
        assert_eq!(at(500.0), DropSlot::Row(2));
    }

    /// A gap between two groups is part of the "between" zone, not a dead
    /// region: the gap sits below the upper group's interior and above the
    /// lower group's top band, both of which resolve to "before the lower".
    #[test]
    fn gap_between_groups_is_between() {
        let pulse = group(0, 0, 0.0, 100.0);
        let trends = group(5, 1, 130.0, 230.0);
        let children = [pulse, trends];
        let band = 0.2; // fraction of each child's height

        // Pulse interior runs to its bottom edge (non-last child).
        assert_eq!(resolve_drop_y(&children, 110.0, band), DropSlot::Row(1));
        // The gap proper.
        assert_eq!(resolve_drop_y(&children, 120.0, band), DropSlot::Row(1));
        // The lower group's top band.
        assert_eq!(resolve_drop_y(&children, 140.0, band), DropSlot::Row(1));
    }

    /// A leaf has no interior: its span splits at the midpoint.
    #[test]
    fn leaf_splits_at_midpoint() {
        let tile = DropChild {
            id: ItemId(9),
            row: 3,
            h: 1,
            top: 100.0,
            bottom: 200.0,
            is_group: false,
        };
        let children = [tile];
        let band = 0.2; // fraction of each child's height
        // Above the midpoint → before the tile.
        assert_eq!(resolve_drop_y(&children, 149.0, band), DropSlot::Row(3));
        // Below the midpoint → after the tile.
        assert_eq!(resolve_drop_y(&children, 151.0, band), DropSlot::Row(4));
    }

    /// A multi-row tile occupies `row..row + h`, so a drop below it lands at
    /// `row + h`, not `row + 1` — the latter would land inside the tile and
    /// collide instead of appending a fresh row beneath it. This is the
    /// "drop a loose tile into a group below its existing rows" case: the
    /// group's 2x2 tiles sit at row 0 spanning rows 0–1, so below them is
    /// row 2.
    #[test]
    fn below_a_tall_tile_clears_its_full_extent() {
        let tile = DropChild {
            id: ItemId(9),
            row: 0,
            h: 2,
            top: 0.0,
            bottom: 100.0,
            is_group: false,
        };
        let children = [tile];
        let band = 0.2;
        // Below the midpoint → below the tile, past its full 2-row extent.
        assert_eq!(resolve_drop_y(&children, 60.0, band), DropSlot::Row(2));
    }

    /// An empty grid resolves to row 0.
    #[test]
    fn empty_grid_is_row_zero() {
        assert_eq!(resolve_drop_y(&[], 50.0, 0.2), DropSlot::Row(0));
    }
}
