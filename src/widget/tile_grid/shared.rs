//! Depth-agnostic helpers shared across tile grid rendering.
//!
//! These are pure geometry and animation utilities with no dependency on
//! the [`TileGrid`] widget's interaction state. They are factored out of
//! [`widget`](super::widget) so the same machinery can drive a single flat
//! grid and, recursively, every sub-grid of a nested layout.

use std::collections::HashMap;

use crate::core::time::Instant;
use crate::core::{Animation, Rectangle, Vector};

use super::engine::Internal;
use super::item_id::ItemId;

/// Minimum cursor travel (in pixels) before a press becomes a drag.
pub(crate) const DRAG_DEADBAND_DISTANCE: f32 = 10.0;

/// Reach (in pixels) of the bottom-right resize wedge.
pub(crate) const RESIZE_CORNER_REACH: f32 = 20.0;

/// Converts a pixel rectangle from an `item_regions`-style tuple to a
/// [`Rectangle`].
pub(crate) fn pixel_rect(region: (f32, f32, f32, f32)) -> Rectangle {
    Rectangle {
        x: region.0,
        y: region.1,
        width: region.2,
        height: region.3,
    }
}

/// Computes pixel regions for the items in an arbitrary engine.
pub(crate) fn compute_regions_for(
    internal: &Internal,
    total_width: f32,
    cell_w: f32,
    cell_h: f32,
    spacing: f32,
) -> Vec<(ItemId, (f32, f32, f32, f32))> {
    internal
        .items()
        .map(|item| {
            let x = f32::from(item.x);
            let y = f32::from(item.y);
            let w = f32::from(item.w);
            let h = f32::from(item.h);

            let px = (x * cell_w + x * spacing).round();
            let py = (y * cell_h + y * spacing).round();
            let pw = (w * cell_w + (w - 1.0) * spacing).round();
            let ph = (h * cell_h + (h - 1.0) * spacing).round();

            // Clamp width to not exceed total_width
            let pw = pw.min(total_width - px);

            (item.id, (px, py, pw, ph))
        })
        .collect()
}

/// Per-item position animations for smooth transitions.
#[derive(Debug, Clone)]
pub(crate) struct ItemAnimations {
    /// Animated X offset for each item (pixels, from layout position).
    offsets_x: HashMap<ItemId, Animation<f32>>,
    /// Animated Y offset for each item (pixels, from layout position).
    offsets_y: HashMap<ItemId, Animation<f32>>,
    /// Last known pixel position for each item (top-left corner).
    last_positions: HashMap<ItemId, (f32, f32)>,
    /// Ghost placeholder opacity animation.
    ghost_opacity: Animation<bool>,
    /// The item that was being dragged when the ghost was last shown.
    ghost_item: Option<ItemId>,
    /// Last known ghost snap position (top-left pixel coords).
    ghost_last_pos: Option<(f32, f32)>,
    /// Animated X offset for the ghost (pixels, from snap position).
    ghost_offset_x: Animation<f32>,
    /// Animated Y offset for the ghost (pixels, from snap position).
    ghost_offset_y: Animation<f32>,
    /// Current time instant for interpolation.
    pub(crate) now: Option<Instant>,
}

impl Default for ItemAnimations {
    fn default() -> Self {
        Self {
            offsets_x: HashMap::new(),
            offsets_y: HashMap::new(),
            last_positions: HashMap::new(),
            ghost_opacity: Animation::new(false),
            ghost_item: None,
            ghost_last_pos: None,
            ghost_offset_x: Animation::new(0.0),
            ghost_offset_y: Animation::new(0.0),
            now: None,
        }
    }
}

impl ItemAnimations {
    fn new_animation(value: f32) -> Animation<f32> {
        Animation::new(value)
            .quick()
            .easing(crate::core::animation::Easing::EaseOut)
    }

    fn new_ghost_animation(value: bool) -> Animation<bool> {
        Animation::new(value)
            .quick()
            .easing(crate::core::animation::Easing::EaseOut)
    }

    /// Returns true if any item animation is in progress.
    pub(crate) fn is_animating(&self, now: Instant) -> bool {
        self.offsets_x.values().any(|anim| anim.is_animating(now))
            || self.offsets_y.values().any(|anim| anim.is_animating(now))
            || self.ghost_opacity.is_animating(now)
            || self.ghost_offset_x.is_animating(now)
            || self.ghost_offset_y.is_animating(now)
    }

    /// Update animations based on new item positions from layout.
    /// `regions` maps ItemId -> (px_x, px_y, pw, ph).
    /// `dragged_id` is the item currently being dragged (should not be animated).
    #[allow(clippy::type_complexity)]
    pub(crate) fn update_positions(
        &mut self,
        regions: &[(ItemId, (f32, f32, f32, f32))],
        dragged_id: Option<ItemId>,
        now: Instant,
    ) {
        for &(id, (px, py, _, _)) in regions {
            // Don't animate the item being dragged
            if dragged_id == Some(id) {
                self.last_positions.insert(id, (px, py));
                continue;
            }

            if let Some(&(old_x, old_y)) = self.last_positions.get(&id) {
                let dx = old_x - px;
                let dy = old_y - py;

                // Only start a new animation if the position actually changed
                // by a meaningful amount (> 0.5px to avoid float noise).
                if dx.abs() > 0.5 || dy.abs() > 0.5 {
                    let anim_x = Self::new_animation(dx).go(0.0, now);
                    let anim_y = Self::new_animation(dy).go(0.0, now);
                    self.offsets_x.insert(id, anim_x);
                    self.offsets_y.insert(id, anim_y);
                }
            }

            self.last_positions.insert(id, (px, py));
        }

        // Clean up stale entries for items that no longer exist.
        let current_ids: std::collections::HashSet<ItemId> =
            regions.iter().map(|(id, _)| *id).collect();
        self.offsets_x.retain(|id, _| current_ids.contains(id));
        self.offsets_y.retain(|id, _| current_ids.contains(id));
        self.last_positions.retain(|id, _| current_ids.contains(id));
    }

    /// Get the current interpolated offset for an item.
    pub(crate) fn get_offset(&self, id: ItemId, now: Instant) -> Vector {
        let x = self
            .offsets_x
            .get(&id)
            .filter(|anim| anim.is_animating(now))
            .map(|anim| anim.interpolate_with(|v| v, now))
            .unwrap_or(0.0);
        let y = self
            .offsets_y
            .get(&id)
            .filter(|anim| anim.is_animating(now))
            .map(|anim| anim.interpolate_with(|v| v, now))
            .unwrap_or(0.0);
        Vector::new(x, y)
    }

    /// Start the ghost fade-in animation.
    pub(crate) fn show_ghost(&mut self, id: ItemId, now: Instant) {
        if self.ghost_item != Some(id) {
            self.ghost_opacity = Self::new_ghost_animation(false).go(true, now);
            self.ghost_item = Some(id);
        }
    }

    /// Hide the ghost (reset state).
    pub(crate) fn hide_ghost(&mut self) {
        if self.ghost_item.is_some() {
            self.ghost_item = None;
            self.ghost_opacity = Self::new_ghost_animation(false);
            self.ghost_last_pos = None;
        }
    }

    /// Get the current ghost opacity (0.0 to 1.0).
    pub(crate) fn ghost_alpha(&self, now: Instant) -> f32 {
        self.ghost_opacity.interpolate(0.0, 1.0, now)
    }

    /// Update ghost position animation when the snap position changes.
    pub(crate) fn update_ghost_position(
        &mut self,
        target: Rectangle,
        now: Instant,
    ) {
        let new_pos = (target.x, target.y);

        if let Some((old_x, old_y)) = self.ghost_last_pos {
            let dx = old_x - new_pos.0;
            let dy = old_y - new_pos.1;

            if dx.abs() > 0.5 || dy.abs() > 0.5 {
                self.ghost_offset_x = Self::new_animation(dx).go(0.0, now);
                self.ghost_offset_y = Self::new_animation(dy).go(0.0, now);
            }
        }

        self.ghost_last_pos = Some(new_pos);
    }

    /// Get the current interpolated offset for the ghost.
    pub(crate) fn ghost_offset(&self, now: Instant) -> Vector {
        let x = if self.ghost_offset_x.is_animating(now) {
            self.ghost_offset_x.interpolate_with(|v| v, now)
        } else {
            0.0
        };
        let y = if self.ghost_offset_y.is_animating(now) {
            self.ghost_offset_y.interpolate_with(|v| v, now)
        } else {
            0.0
        };
        Vector::new(x, y)
    }
}

/// Clips a ghost rectangle to avoid overlapping with items that are
/// animating away from the ghost's area.
///
/// Each entry in `animating_items` is `(layout_bounds, animation_offset)`,
/// where `layout_bounds` is the item's final (layout) position, and
/// `animation_offset` is the current visual offset from that position (the
/// offset starts large and converges to zero as the animation completes).
///
/// The returned rectangle is the ghost shrunk so it never visually overlaps
/// any of the animating items.
pub(crate) fn clip_ghost_for_animating_items(
    ghost: Rectangle,
    animating_items: &[(Rectangle, Vector)],
) -> Rectangle {
    let mut bounds = ghost;

    for &(layout_rect, offset) in animating_items {
        if offset.x == 0.0 && offset.y == 0.0 {
            continue;
        }

        // The item's visual rectangle: layout position + current offset.
        let visual = Rectangle {
            x: layout_rect.x + offset.x,
            y: layout_rect.y + offset.y,
            width: layout_rect.width,
            height: layout_rect.height,
        };

        // Skip if no overlap between current ghost bounds and this item.
        if bounds.y >= visual.y + visual.height
            || bounds.y + bounds.height <= visual.y
            || bounds.x >= visual.x + visual.width
            || bounds.x + bounds.width <= visual.x
        {
            continue;
        }

        // Clip vertically based on offset direction.
        if offset.y < 0.0 {
            // Item visual is above its layout position, sliding DOWN.
            // It occupies space in the lower portion of the ghost area.
            // Reveal the ghost from the top: clip the bottom edge.
            let available = (visual.y - bounds.y).max(0.0);
            bounds.height = bounds.height.min(available);
        } else if offset.y > 0.0 {
            // Item visual is below its layout position, sliding UP.
            // It occupies space in the upper portion of the ghost area.
            // Reveal the ghost from the bottom: clip the top edge.
            let new_top = (visual.y + visual.height)
                .max(bounds.y)
                .min(bounds.y + bounds.height);
            bounds.height -= new_top - bounds.y;
            bounds.y = new_top;
        }

        // Clip horizontally based on offset direction.
        if offset.x < 0.0 {
            // Item visual is left of its layout position, sliding RIGHT.
            // Reveal the ghost from the left: clip the right edge.
            let available = (visual.x - bounds.x).max(0.0);
            bounds.width = bounds.width.min(available);
        } else if offset.x > 0.0 {
            // Item visual is right of its layout position, sliding LEFT.
            // Reveal the ghost from the right: clip the left edge.
            let new_left = (visual.x + visual.width)
                .max(bounds.x)
                .min(bounds.x + bounds.width);
            bounds.width -= new_left - bounds.x;
            bounds.x = new_left;
        }
    }

    // Ensure non-negative dimensions.
    bounds.width = bounds.width.max(0.0);
    bounds.height = bounds.height.max(0.0);

    bounds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn vec2(x: f32, y: f32) -> Vector {
        Vector::new(x, y)
    }

    #[test]
    fn ghost_unchanged_when_no_animating_items() {
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let result = clip_ghost_for_animating_items(ghost, &[]);
        assert_eq!(result, ghost);
    }

    #[test]
    fn ghost_unchanged_when_items_not_overlapping() {
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        // Item far below ghost, animating but not overlapping.
        let items = [(rect(0.0, 500.0, 280.0, 200.0), vec2(0.0, -50.0))];
        let result = clip_ghost_for_animating_items(ghost, &items);
        assert_eq!(result, ghost);
    }

    #[test]
    fn ghost_grows_from_top_as_item_slides_down() {
        // Ghost at (0, 0, 280, 200).
        // Item displaced from y=0 to y=208 (layout).
        // offset_y starts at -208 (visual at y=0), goes to 0 (visual at y=208).
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 208.0, 280.0, 200.0);

        // At animation start: offset_y = -208, visual at y=0 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -208.0))],
        );
        assert_eq!(result.height, 0.0);
        assert_eq!(result.y, 0.0);

        // At midpoint: offset_y = -104, visual at y=104.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -104.0))],
        );
        assert_eq!(result.height, 104.0);
        assert_eq!(result.y, 0.0);

        // Near end: offset_y = -8, visual at y=200 — no overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -8.0))],
        );
        assert_eq!(result.height, 200.0);
        assert_eq!(result.y, 0.0);
    }

    #[test]
    fn ghost_grows_from_bottom_as_item_slides_up() {
        // Ghost at (0, 208, 280, 200) — from y=208 to y=408.
        // Item displaced from y=208 to y=0 (layout).
        // offset_y starts at 208 (visual at y=208), goes to 0 (visual at y=0).
        let ghost = rect(0.0, 208.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 0.0, 280.0, 200.0);

        // At animation start: offset_y = 208, visual at y=208 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, 208.0))],
        );
        assert_eq!(result.height, 0.0);

        // At midpoint: offset_y = 104, visual at y=104 — bottom of visual is 304.
        // Ghost from 208..408, visual from 104..304. Overlap = 208..304.
        // Ghost should show 304..408, height = 104.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, 104.0))],
        );
        assert!((result.y - 304.0).abs() < f32::EPSILON);
        assert!((result.height - 104.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ghost_grows_from_left_as_item_slides_right() {
        // Ghost at (0, 0, 280, 200).
        // Item displaced from x=0 to x=300 (layout).
        // offset_x starts at -300 (visual at x=0), goes to 0 (visual at x=300).
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(300.0, 0.0, 280.0, 200.0);

        // At animation start: offset_x = -300, visual at x=0 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(-300.0, 0.0))],
        );
        assert_eq!(result.width, 0.0);
        assert_eq!(result.x, 0.0);

        // At midpoint: offset_x = -150, visual at x=150.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(-150.0, 0.0))],
        );
        assert_eq!(result.width, 150.0);
        assert_eq!(result.x, 0.0);
    }

    #[test]
    fn ghost_grows_from_right_as_item_slides_left() {
        // Ghost at (300, 0, 280, 200).
        // Item displaced from x=300 to x=0 (layout).
        // offset_x starts at 300 (visual at x=300), goes to 0 (visual at x=0).
        let ghost = rect(300.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 0.0, 280.0, 200.0);

        // At animation start: offset_x = 300, visual at x=300 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(300.0, 0.0))],
        );
        assert_eq!(result.width, 0.0);

        // At midpoint: offset_x = 150, visual at x=150 — visual right = 430.
        // Ghost from x=300..580. Overlap = 300..430.
        // Ghost should show 430..580, width = 150.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(150.0, 0.0))],
        );
        assert!((result.x - 430.0).abs() < f32::EPSILON);
        assert!((result.width - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ghost_clipped_by_multiple_items() {
        // Ghost at (0, 0, 280, 400).
        // Item A sliding down from top of ghost.
        // Item B sliding up from bottom of ghost.
        let ghost = rect(0.0, 0.0, 280.0, 400.0);

        // Item A: layout at y=200, offset_y = -100, visual at y=100.
        // Clips ghost bottom to y=100.
        let item_a = (rect(0.0, 200.0, 280.0, 200.0), vec2(0.0, -100.0));

        // Item B: layout at y=-200, offset_y = 100, visual at y=-100, bottom=100.
        // Clips ghost top to y=100. But after item A already clipped bottom
        // to height=100 (ghost goes from 0..100), item B visual bottom is 100
        // which equals the ghost bottom, so overlap check: ghost 0..100,
        // visual -100..100 — they overlap. Clip top: new_top = 100.
        // ghost height = 0.
        let item_b = (rect(0.0, -200.0, 280.0, 200.0), vec2(0.0, 100.0));

        let result = clip_ghost_for_animating_items(ghost, &[item_a, item_b]);
        assert_eq!(result.height, 0.0);
    }

    #[test]
    fn ghost_full_size_when_item_animation_complete() {
        // Item has zero offset — animation finished.
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let items = [(rect(0.0, 208.0, 280.0, 200.0), vec2(0.0, 0.0))];
        let result = clip_ghost_for_animating_items(ghost, &items);
        assert_eq!(result, ghost);
    }
}
