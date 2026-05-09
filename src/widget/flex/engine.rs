//! Pure CSS-flex layout helpers and the [`resolve`] driver.
//!
//! All items in this module are `pub(crate)` — the engine is internal
//! to the `flex` namespace. Public consumers go through `FlexChild`,
//! `Row`, and `Column` (added in later phases).
//!
//! The driver is parameterized over a layouter callback so the math
//! can be exercised without an iced widget tree. See the test module
//! for the full set of shape and edge-case fixtures.
//!
//! The driver mirrors `iced_core::layout::flex::resolve` pass-for-pass
//! — see the inline comments on [`resolve`] for the bucket structure.

use crate::core::layout::{Limits, Node};
use crate::core::{Length, Padding, Point, Size};

use super::alignment::{AlignItems, AlignSelf, Axis, Justify};
use super::child::{Basis, Properties};

/// Solves CSS flex-grow / flex-shrink given each item's resolved base
/// main size.
///
/// The returned `Vec<f32>` has the same length as `base_sizes` and
/// gives the final main-axis size for each item.
///
/// `free_space` is `container_main - sum(base) - gap_total`. Positive
/// free-space triggers grow distribution; negative free-space triggers
/// shrink distribution.
///
/// **Grow** distributes surplus space proportionally to each item's
/// `grow` factor. If every `grow` is zero, falls back to the
/// `fill_main` factor so plain `Length::Fill` items still split surplus
/// space the way iced does today.
///
/// **Shrink** scales by `shrink * basis` per CSS — larger items absorb
/// more of the deficit. Items with `shrink == 0` keep their base size
/// even when the container overflows.
///
/// [`resolve`]'s grow path is inlined against the unmeasured-items
/// filter (since it interleaves with `layout_item` calls per child);
/// the shrink path calls back into this helper after Pass 4 to
/// redistribute deficit across already-measured items.
pub(crate) fn solve_main_sizes(
    base_sizes: &[f32],
    props: &[Properties],
    free_space: f32,
) -> Vec<f32> {
    debug_assert_eq!(base_sizes.len(), props.len());

    let mut out: Vec<f32> = base_sizes.to_vec();

    if base_sizes.is_empty() || free_space == 0.0 {
        return out;
    }

    if free_space > 0.0 {
        // Grow distribution. Prefer explicit `grow` factors; if every
        // grow is zero, fall back to `fill_main` so untouched fluid
        // items continue to expand as in upstream iced.
        let total_grow: f32 = props.iter().map(|p| p.grow).sum();

        if total_grow > 0.0 {
            for (i, p) in props.iter().enumerate() {
                if p.grow > 0.0 {
                    let share = free_space * p.grow / total_grow;
                    out[i] = base_sizes[i] + share;
                }
            }
        } else {
            let total_fill: u32 =
                props.iter().map(|p| u32::from(p.fill_main)).sum();
            if total_fill > 0 {
                let total_fill_f = total_fill as f32;
                for (i, p) in props.iter().enumerate() {
                    if p.fill_main > 0 {
                        let share =
                            free_space * f32::from(p.fill_main) / total_fill_f;
                        out[i] = base_sizes[i] + share;
                    }
                }
            }
        }
    } else {
        // Shrink distribution. CSS scales by shrink * basis.
        let weights: Vec<f32> = props
            .iter()
            .zip(base_sizes.iter())
            .map(|(p, base)| p.shrink * base)
            .collect();
        let total_weight: f32 = weights.iter().sum();

        if total_weight > 0.0 {
            // free_space is negative; deficit is its absolute value.
            let deficit = -free_space;
            for i in 0..base_sizes.len() {
                if weights[i] > 0.0 {
                    let share = deficit * weights[i] / total_weight;
                    out[i] = (base_sizes[i] - share).max(0.0);
                }
            }
        }
        // total_weight == 0: every item has shrink=0 (or basis=0); the
        // container overflows and items keep their base sizes.
    }

    out
}

/// Computes `justify-content` offsets.
///
/// Returns `(initial, gap_extra)` where `initial` is the offset before
/// the first item and `gap_extra` is the additional spacing between
/// adjacent items beyond the configured `spacing`.
///
/// `count` is the number of items; `leftover` is `container_main -
/// sum(item_main_sizes) - gap_total` (clamped to non-negative by the
/// caller — negative leftover is an overflow case where justification
/// has no space to distribute).
///
/// Returns `(0.0, 0.0)` when `count == 0` or `leftover <= 0` (both
/// degenerate cases — there is nothing to distribute).
pub(crate) fn justify_offsets(
    count: usize,
    leftover: f32,
    justify: Justify,
) -> (f32, f32) {
    if count == 0 || leftover <= 0.0 {
        return (0.0, 0.0);
    }

    match justify {
        Justify::Start => (0.0, 0.0),
        Justify::End => (leftover, 0.0),
        Justify::Center => (leftover / 2.0, 0.0),
        Justify::SpaceBetween => {
            if count < 2 {
                (0.0, 0.0)
            } else {
                (0.0, leftover / (count - 1) as f32)
            }
        }
        Justify::SpaceAround => {
            // Each item has half a unit of padding on each side. The
            // gap between two adjacent items is therefore one full
            // unit, and each end has half a unit.
            let unit = leftover / count as f32;
            (unit / 2.0, unit)
        }
        Justify::SpaceEvenly => {
            // count + 1 equal gaps (start, between each pair, end).
            let unit = leftover / (count + 1) as f32;
            (unit, unit)
        }
    }
}

/// Computes the cross-axis offset for a single item.
///
/// `cross_size` is the item's measured cross length; `container_cross`
/// is the container's resolved cross length. `align_self` is the
/// per-item override; `Auto` defers to `container_align`.
///
/// `Stretch` returns 0.0 (the item has been laid out to the full
/// `container_cross` already, so no offset is needed). The driver is
/// responsible for actually stretching.
pub(crate) fn cross_offset(
    cross_size: f32,
    container_cross: f32,
    container_align: AlignItems,
    align_self: AlignSelf,
) -> f32 {
    let resolved = align_self.resolve(container_align);
    let space = (container_cross - cross_size).max(0.0);

    match resolved {
        AlignItems::Start | AlignItems::Stretch => 0.0,
        AlignItems::End => space,
        AlignItems::Center => space / 2.0,
    }
}

/// Resolves a CSS-flex layout, calling `layout_item(idx, &limits)` for
/// each child as needed.
///
/// The driver mirrors `iced_core::layout::flex::resolve`'s 5-pass
/// structure so cross-axis compression behaves identically:
///
/// 1. **Pass 1 — non-fluid main**: lay out each item whose main is
///    non-fluid, *or* whose main is fluid but the container is
///    main-compressed. Items with `Basis::Pixels(p)` skip the call
///    and adopt `p` as their base size directly. Cross-fluid items
///    that need the cross-compress treatment are skipped here.
/// 2. **Pass 2 — cross-compress deferred bucket** (when the container
///    is cross-compressed and at least one child is cross-fluid):
///    lay out cross-fluid + non-fluid-main items using the maximum
///    cross size measured in Pass 1. Fixed-main + cross-fluid items
///    are deferred to Pass 4.
/// 3. **Pass 3 — fluid main**: distribute leftover main space using
///    [`solve_main_sizes`] and lay out each grow item at its assigned
///    size.
/// 4. **Pass 4 — deferred fix-up** (paired with Pass 2): lay out the
///    items deferred in Pass 2 now that Pass 3 has finalized the
///    cross length.
/// 5. **Pass 5 — position**: compute justify offsets via
///    [`justify_offsets`], then walk items in forward or reverse
///    order, positioning each with its cross offset via
///    [`cross_offset`]. Stretch items are stretched to
///    `container_cross`.
///
/// Worst case is one layout call per child (Pass 1 and Pass 3 are
/// disjoint sets), matching iced's existing upper bound.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve<F>(
    axis: Axis,
    limits: &Limits,
    width: Length,
    height: Length,
    padding: Padding,
    gap: f32,
    justify: Justify,
    align: AlignItems,
    reverse: bool,
    props: &[Properties],
    mut layout_item: F,
) -> Node
where
    F: FnMut(usize, &Limits) -> Node,
{
    let limits = limits.width(width).height(height).shrink(padding);
    let count = props.len();

    if count == 0 {
        let intrinsic = Size::new(0.0, 0.0);
        let size = limits.resolve(width, height, intrinsic);
        return Node::with_children(size.expand(padding), Vec::new());
    }

    let total_gap = gap * count.saturating_sub(1) as f32;
    let max_main = axis.main(limits.max());
    let max_cross = axis.cross(limits.max());

    let (main_compress, cross_compress) = {
        let compression = limits.compression();
        axis.pack(compression.width, compression.height)
    };

    // The per-child compression flag must mirror iced exactly: only
    // the main-axis compression bit is propagated; the cross bit is
    // false for the per-child limits, since the engine drives cross
    // sizing explicitly.
    let compression = {
        let (cx, cy) = axis.pack(main_compress, false);
        Size::new(cx, cy)
    };

    let mut nodes: Vec<Node> = (0..count).map(|_| Node::default()).collect();
    let mut base_sizes: Vec<f32> = vec![0.0; count];
    // Pass 1 sets `measured[i] = true` when an item's final layout has
    // already been recorded in `nodes[i]`. Pass 3 must skip them.
    let mut measured: Vec<bool> = vec![false; count];

    let mut cross = if cross_compress { 0.0 } else { max_cross };
    let mut available = max_main - total_gap;
    let mut some_fill_cross = false;

    // ------------------------------------------------------------------
    // Pass 1 — non-fluid main (or main-compressed container).
    // ------------------------------------------------------------------
    for (i, p) in props.iter().enumerate() {
        let fill_main = p.fill_main_factor();
        let fill_cross = p.fill_cross;

        // The condition mirrors iced's flex.rs:110 — lay out items
        // whose main is fixed (or container is main-compressed),
        // skipping any that are cross-fluid when the container itself
        // is cross-compressed (those are handled in Pass 2 or 4).
        let main_eligible = main_compress || fill_main == 0;
        let cross_eligible = !cross_compress || fill_cross == 0;

        if main_eligible && cross_eligible {
            // Pixel-basis items: we know the main size up front, so
            // we lay them out at (b, cross-budget) and record. Their
            // base size is the basis itself. The cross-axis budget
            // depends on whether they're cross-fluid (use Pass 1's
            // running max) or not (use the full `max_cross`).
            if let Basis::Pixels(b) = p.basis
                && fill_main == 0
            {
                let (mw, mh) = axis
                    .pack(b, if fill_cross == 0 { max_cross } else { cross });
                let item_limits = Limits::with_compression(
                    Size::ZERO,
                    Size::new(mw, mh),
                    compression,
                );
                let node = layout_item(i, &item_limits);
                let item_main = axis.main(node.size());
                base_sizes[i] = item_main;
                available -= item_main;
                cross = cross.max(axis.cross(node.size()));
                nodes[i] = node;
                measured[i] = true;
                continue;
            }

            let (mw, mh) = axis.pack(
                available,
                if fill_cross == 0 { max_cross } else { cross },
            );
            let item_limits = Limits::with_compression(
                Size::ZERO,
                Size::new(mw, mh),
                compression,
            );
            let node = layout_item(i, &item_limits);
            let item_size = node.size();

            base_sizes[i] = axis.main(item_size);
            available -= base_sizes[i];
            cross = cross.max(axis.cross(item_size));

            nodes[i] = node;
            measured[i] = true;
        } else if fill_cross != 0 {
            some_fill_cross = true;
        }
    }

    // ------------------------------------------------------------------
    // Pass 2 — cross-compress deferred bucket.
    //
    // Lay out cross-fluid items with non-fluid main using the maximum
    // cross obtained in Pass 1. Fixed-main + cross-fluid items are
    // deferred to Pass 4 so they can use Pass 3's final cross.
    // ------------------------------------------------------------------
    if cross_compress && some_fill_cross {
        for (i, p) in props.iter().enumerate() {
            if measured[i] {
                continue;
            }
            let fill_main = p.fill_main_factor();
            let fill_cross = p.fill_cross;

            if (main_compress || fill_main == 0) && fill_cross != 0 {
                // Fixed-main + cross-fluid: defer to Pass 4.
                if let Basis::Pixels(b) = p.basis {
                    base_sizes[i] = b;
                    available -= b;
                    continue;
                }

                let (mw, mh) = axis.pack(available, cross);
                let item_limits = Limits::with_compression(
                    Size::ZERO,
                    Size::new(mw, mh),
                    compression,
                );
                let node = layout_item(i, &item_limits);
                let item_size = node.size();

                base_sizes[i] = axis.main(item_size);
                available -= base_sizes[i];
                cross = cross.max(axis.cross(item_size));

                nodes[i] = node;
                measured[i] = true;
            }
        }
    }

    let remaining = available.max(0.0);

    // ------------------------------------------------------------------
    // Pass 3 — fluid main.
    //
    // Distribute leftover main-axis space among items with main-fluid
    // sizes. Skipped entirely when the container is main-compressed
    // (those items had their main determined in Pass 1 already).
    // ------------------------------------------------------------------
    if !main_compress {
        let total_grow: f32 = props
            .iter()
            .enumerate()
            .filter(|(i, _)| !measured[*i])
            .map(|(_, p)| p.grow)
            .sum();
        let total_fill: u32 = props
            .iter()
            .enumerate()
            .filter(|(i, _)| !measured[*i])
            .map(|(_, p)| u32::from(p.fill_main))
            .sum();

        for (i, p) in props.iter().enumerate() {
            if measured[i] {
                continue;
            }

            let fill_cross = p.fill_cross;

            // Determine this item's share of `remaining`. Prefer
            // explicit grow; fall back to fill_main factor when no
            // grow is configured (preserves iced's FillPortion
            // semantics for plain Length::Fill items).
            let share_main = if total_grow > 0.0 && p.grow > 0.0 {
                remaining * p.grow / total_grow
            } else if total_grow == 0.0 && total_fill > 0 && p.fill_main > 0 {
                remaining * f32::from(p.fill_main) / total_fill as f32
            } else if p.fill_main > 0 {
                // No grow, but this item is fluid and others have
                // grow — distribute zero to non-grow fluid items.
                0.0
            } else {
                // No fluid hint at all — nothing to do here. (This
                // case should not arise: if an item is unmeasured at
                // this point, it must have been fluid in the main
                // axis.)
                continue;
            };

            // Mirror iced's flex.rs:195-205 NaN/infinity guard.
            let share_main = if share_main.is_nan() {
                f32::INFINITY
            } else {
                share_main
            };
            let min_main = if share_main.is_infinite() {
                0.0
            } else {
                share_main
            };

            let (min_w, min_h) = axis.pack(min_main, 0.0);
            let (max_w, max_h) = axis.pack(
                share_main,
                if fill_cross == 0 { max_cross } else { cross },
            );
            let item_limits = Limits::with_compression(
                Size::new(min_w, min_h),
                Size::new(max_w, max_h),
                compression,
            );
            let node = layout_item(i, &item_limits);
            cross = cross.max(axis.cross(node.size()));

            nodes[i] = node;
            measured[i] = true;
        }
    }

    // ------------------------------------------------------------------
    // Pass 4 — deferred fix-up (paired with Pass 2).
    //
    // Items that are fixed in the main axis but fluid in the cross
    // axis: re-lay out now that Pass 3 has finalized `cross`.
    // ------------------------------------------------------------------
    if cross_compress && some_fill_cross {
        for (i, p) in props.iter().enumerate() {
            let fill_cross = p.fill_cross;
            if fill_cross == 0 {
                continue;
            }
            // Only fixed-main items are deferred here. Identified by:
            // unmeasured at this point AND has Basis::Pixels (or, by
            // construction, was deferred in Pass 2 when we hit the
            // `Basis::Pixels` branch).
            let Basis::Pixels(b) = p.basis else {
                continue;
            };
            if measured[i] {
                continue;
            }

            let (mw, mh) = axis.pack(b, cross);
            let item_limits = Limits::new(Size::ZERO, Size::new(mw, mh));
            let node = layout_item(i, &item_limits);
            cross = cross.max(axis.cross(node.size()));
            base_sizes[i] = axis.main(node.size());
            nodes[i] = node;
            measured[i] = true;
        }
    }

    // ------------------------------------------------------------------
    // Pass S — shrink overflow.
    //
    // CSS `flex-shrink` distributes a main-axis deficit weighted by
    // each item's `basis * shrink`. Pass 1 commits Basis::Pixels items
    // to their basis size, so if their total exceeds the container's
    // main budget we need to redistribute the deficit and re-lay out
    // the shrunk items. Pass 3's grow path can never produce overflow
    // (it only allocates leftover), so this only kicks in when items
    // overflow purely on their bases.
    //
    // Skipped when the container is main-compressed — a Shrink-main
    // container resolves to its intrinsic, so there's no defined
    // "container main" to shrink against.
    if !main_compress {
        let main_after_passes: Vec<f32> =
            (0..count).map(|i| axis.main(nodes[i].size())).collect();
        let used: f32 = main_after_passes.iter().sum();
        let free_space = max_main - total_gap - used;
        if free_space < 0.0 {
            // `solve_main_sizes` handles both grow and shrink. Negative
            // free_space takes the shrink branch, scaling by
            // `basis * shrink` per CSS.
            let final_sizes =
                solve_main_sizes(&main_after_passes, props, free_space);
            for (i, p) in props.iter().enumerate() {
                if p.shrink <= 0.0 {
                    continue;
                }
                let delta = main_after_passes[i] - final_sizes[i];
                if delta.abs() < 0.5 {
                    continue;
                }
                // Re-lay out at the shrunk main size. Cross max stays
                // at the running `cross` so a stretched item keeps
                // its cross-axis dimension across the second layout.
                let cross_max =
                    if p.fill_cross == 0 { max_cross } else { cross };
                let (mw, mh) = axis.pack(final_sizes[i], cross_max);
                let min = axis.pack(final_sizes[i], 0.0);
                let item_limits = Limits::with_compression(
                    Size::new(min.0, min.1),
                    Size::new(mw, mh),
                    compression,
                );
                let node = layout_item(i, &item_limits);
                nodes[i] = node;
                cross = cross.max(axis.cross(nodes[i].size()));
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 5 — position.
    // ------------------------------------------------------------------

    // Sum the main sizes after layout. Stretch items keep their
    // measured cross; alignment is per-item.
    let main_used: f32 = nodes.iter().map(|n| axis.main(n.size())).sum();
    let total_main = main_used + total_gap;

    // Resolve container size now so positioning uses the *resolved*
    // bounds, not the limit's max. A `width: Shrink` container
    // resolves to its intrinsic main — leftover is zero, and
    // distribution-style justifications (SpaceBetween/Around/Evenly,
    // End, Center) correctly distribute nothing rather than pushing
    // items past the resolved right edge using `max_main`.
    let (intrinsic_w, intrinsic_h) = axis.pack(total_main, cross);
    let intrinsic = Size::new(intrinsic_w, intrinsic_h);
    let resolved_size = limits.resolve(width, height, intrinsic);
    let resolved_main = axis.main(resolved_size);
    let leftover = (resolved_main - total_main).max(0.0);

    // CSS `flex-direction: row-reverse` swaps the visual main-start
    // and main-end. Implement by flipping Justify::Start ↔ End for
    // the offset computation and iterating items in reverse source
    // order. SpaceBetween / SpaceAround / SpaceEvenly are symmetric.
    let effective_justify = if reverse {
        match justify {
            Justify::Start => Justify::End,
            Justify::End => Justify::Start,
            Justify::Center
            | Justify::SpaceBetween
            | Justify::SpaceAround
            | Justify::SpaceEvenly => justify,
        }
    } else {
        justify
    };

    let (initial, gap_extra) =
        justify_offsets(count, leftover, effective_justify);
    let pad = axis.pack(padding.left, padding.top);
    let mut main_cursor = pad.0 + initial;

    // Order to iterate the *positions* in. The items themselves stay
    // in source order in the returned `nodes` Vec — only the
    // assignment of position-slots changes when reversed.
    let order: Vec<usize> = if reverse {
        (0..count).rev().collect()
    } else {
        (0..count).collect()
    };

    for (slot, &i) in order.iter().enumerate() {
        if slot > 0 {
            main_cursor += gap + gap_extra;
        }
        let node_size = nodes[i].size();
        let item_main = axis.main(node_size);
        let item_cross = axis.cross(node_size);

        // Stretch is realized by laying the item out against the full
        // container cross length in earlier passes, so the cross
        // offset for Stretch is always zero. Other variants delegate
        // to `cross_offset`.
        let resolved_align = props[i].align_self.resolve(align);
        let cross_off = match resolved_align {
            AlignItems::Stretch => 0.0,
            AlignItems::Start | AlignItems::End | AlignItems::Center => {
                cross_offset(item_cross, cross, align, props[i].align_self)
            }
        };

        let (x, y) = axis.pack(main_cursor, pad.1 + cross_off);
        nodes[i].move_to_mut(Point::new(x, y));

        main_cursor += item_main;
    }

    Node::with_children(resolved_size.expand(padding), nodes)
}

impl Properties {
    /// Returns the effective main-axis fill factor for engine
    /// classification. Non-zero `grow` overrides the implicit
    /// `fill_main` hint — a child with explicit grow is always
    /// considered main-fluid regardless of its inner widget's `Length`.
    fn fill_main_factor(&self) -> u16 {
        if self.grow > 0.0 {
            self.fill_main.max(1)
        } else {
            self.fill_main
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Pixels, Size};
    use std::cell::RefCell;

    // ----- Helpers -----

    fn props_default(n: usize) -> Vec<Properties> {
        (0..n).map(|_| Properties::default()).collect()
    }

    fn p_grow(grow: f32) -> Properties {
        Properties {
            grow,
            ..Properties::default()
        }
    }

    fn p_shrink(shrink: f32) -> Properties {
        Properties {
            shrink,
            ..Properties::default()
        }
    }

    fn p_fill_main(factor: u16) -> Properties {
        Properties {
            fill_main: factor,
            basis: Basis::Pixels(0.0),
            ..Properties::default()
        }
    }

    fn p_pixels(p: f32) -> Properties {
        Properties {
            basis: Basis::Pixels(p),
            ..Properties::default()
        }
    }

    fn limits(max_w: f32, max_h: f32) -> Limits {
        Limits::new(Size::ZERO, Size::new(max_w, max_h))
    }

    /// A layouter that returns a fixed Size for each index, recording
    /// the call order so tests can assert pass ordering.
    struct Fixture {
        sizes: Vec<Size>,
        order: RefCell<Vec<usize>>,
    }

    impl Fixture {
        fn new(sizes: Vec<Size>) -> Self {
            Self {
                sizes,
                order: RefCell::new(Vec::new()),
            }
        }

        fn layouter(&self) -> impl FnMut(usize, &Limits) -> Node + '_ {
            move |idx, _limits| {
                self.order.borrow_mut().push(idx);
                Node::new(self.sizes[idx])
            }
        }

        /// Layouter that clamps each fixture size to the limits' max,
        /// so a re-layout pass at a smaller main constraint yields a
        /// smaller node — modelling what a real iced container does.
        fn honest_layouter(&self) -> impl FnMut(usize, &Limits) -> Node + '_ {
            move |idx, limits| {
                self.order.borrow_mut().push(idx);
                let want = self.sizes[idx];
                let max = limits.max();
                Node::new(Size::new(
                    want.width.min(max.width),
                    want.height.min(max.height),
                ))
            }
        }
    }

    // ===== solve_main_sizes =====

    #[test]
    fn solve_all_fixed_zero_free() {
        let base = vec![100.0, 200.0, 50.0];
        let props = props_default(3);
        let out = solve_main_sizes(&base, &props, 0.0);
        assert_eq!(out, base);
    }

    #[test]
    fn solve_grow_one_item() {
        let base = vec![100.0, 200.0];
        let mut props = props_default(2);
        props[0].grow = 1.0;
        let out = solve_main_sizes(&base, &props, 100.0);
        assert_eq!(out, vec![200.0, 200.0]);
    }

    #[test]
    fn solve_grow_ratio_one_two_three() {
        let base = vec![0.0, 0.0, 0.0];
        let props = vec![p_grow(1.0), p_grow(2.0), p_grow(3.0)];
        let out = solve_main_sizes(&base, &props, 60.0);
        assert_eq!(out, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn solve_shrink_proportional_to_basis_times_shrink() {
        let base = vec![100.0, 200.0];
        let props = props_default(2); // shrink=1 each
        // Deficit 90 split by basis*shrink => 100:200 ratio.
        // 100 absorbs 30; 200 absorbs 60.
        let out = solve_main_sizes(&base, &props, -90.0);
        assert!((out[0] - 70.0).abs() < 1e-3);
        assert!((out[1] - 140.0).abs() < 1e-3);
    }

    #[test]
    fn solve_all_shrink_zero_keeps_base_overflow() {
        let base = vec![100.0, 200.0];
        let props = vec![p_shrink(0.0), p_shrink(0.0)];
        let out = solve_main_sizes(&base, &props, -50.0);
        assert_eq!(out, base);
    }

    #[test]
    fn solve_fill_fallback_no_grow() {
        let base = vec![0.0, 0.0];
        let props = vec![p_fill_main(1), p_fill_main(2)];
        let out = solve_main_sizes(&base, &props, 90.0);
        assert!((out[0] - 30.0).abs() < 1e-3);
        assert!((out[1] - 60.0).abs() < 1e-3);
    }

    #[test]
    fn solve_zero_items() {
        let out = solve_main_sizes(&[], &[], 100.0);
        assert!(out.is_empty());
    }

    #[test]
    fn solve_one_item_grows() {
        let base = vec![20.0];
        let props = vec![p_grow(1.0)];
        let out = solve_main_sizes(&base, &props, 80.0);
        assert_eq!(out, vec![100.0]);
    }

    #[test]
    fn solve_mixed_grow_zero_and_one() {
        // Only the grow=1 item gets the extra; grow=0 stays put.
        let base = vec![50.0, 50.0, 50.0];
        let props = vec![
            Properties::default(), // grow=0
            p_grow(1.0),
            Properties::default(), // grow=0
        ];
        let out = solve_main_sizes(&base, &props, 60.0);
        assert_eq!(out, vec![50.0, 110.0, 50.0]);
    }

    #[test]
    fn solve_one_shrink_zero_others_share_deficit() {
        let base = vec![100.0, 100.0, 100.0];
        let props = vec![p_shrink(0.0), p_shrink(1.0), p_shrink(1.0)];
        // Deficit 60 split between items 1 and 2 by 100:100 weights.
        let out = solve_main_sizes(&base, &props, -60.0);
        assert_eq!(out[0], 100.0);
        assert!((out[1] - 70.0).abs() < 1e-3);
        assert!((out[2] - 70.0).abs() < 1e-3);
    }

    #[test]
    fn solve_shrink_clamps_to_zero_under_extreme_deficit() {
        let base = vec![10.0, 10.0];
        let props = props_default(2);
        let out = solve_main_sizes(&base, &props, -100.0);
        assert_eq!(out, vec![0.0, 0.0]);
    }

    // ===== justify_offsets =====

    #[test]
    fn justify_start_zero_leftover() {
        for c in 1..=3 {
            assert_eq!(justify_offsets(c, 0.0, Justify::Start), (0.0, 0.0));
        }
    }

    #[test]
    fn justify_count_zero_returns_zeros() {
        for j in [
            Justify::Start,
            Justify::End,
            Justify::Center,
            Justify::SpaceBetween,
            Justify::SpaceAround,
            Justify::SpaceEvenly,
        ] {
            assert_eq!(justify_offsets(0, 100.0, j), (0.0, 0.0));
        }
    }

    #[test]
    fn justify_end_pushes_initial_to_leftover() {
        assert_eq!(justify_offsets(3, 90.0, Justify::End), (90.0, 0.0));
    }

    #[test]
    fn justify_center_halves_initial() {
        assert_eq!(justify_offsets(2, 90.0, Justify::Center), (45.0, 0.0));
    }

    #[test]
    fn justify_space_between_three_items() {
        // Two gaps absorb 90 => 45 each, no initial.
        assert_eq!(
            justify_offsets(3, 90.0, Justify::SpaceBetween),
            (0.0, 45.0)
        );
    }

    #[test]
    fn justify_space_between_one_item_falls_back_to_start() {
        assert_eq!(justify_offsets(1, 90.0, Justify::SpaceBetween), (0.0, 0.0));
    }

    #[test]
    fn justify_space_around_three_items() {
        // unit = 90/3 = 30; initial = 15; gap = 30.
        let (init, gap) = justify_offsets(3, 90.0, Justify::SpaceAround);
        assert!((init - 15.0).abs() < 1e-3);
        assert!((gap - 30.0).abs() < 1e-3);
    }

    #[test]
    fn justify_space_evenly_two_items() {
        // unit = 90/3 = 30 (3 gaps: start, between, end).
        let (init, gap) = justify_offsets(2, 90.0, Justify::SpaceEvenly);
        assert!((init - 30.0).abs() < 1e-3);
        assert!((gap - 30.0).abs() < 1e-3);
    }

    // ===== cross_offset =====

    #[test]
    fn cross_offset_start_is_zero() {
        let off = cross_offset(20.0, 100.0, AlignItems::Start, AlignSelf::Auto);
        assert_eq!(off, 0.0);
    }

    #[test]
    fn cross_offset_end_pushes_to_far_edge() {
        let off = cross_offset(20.0, 100.0, AlignItems::End, AlignSelf::Auto);
        assert_eq!(off, 80.0);
    }

    #[test]
    fn cross_offset_center_halves_space() {
        let off =
            cross_offset(20.0, 100.0, AlignItems::Center, AlignSelf::Auto);
        assert_eq!(off, 40.0);
    }

    #[test]
    fn cross_offset_self_overrides_container() {
        let off = cross_offset(20.0, 100.0, AlignItems::Start, AlignSelf::End);
        assert_eq!(off, 80.0);
    }

    #[test]
    fn cross_offset_stretch_no_offset_with_equal_sizes() {
        let off =
            cross_offset(100.0, 100.0, AlignItems::Stretch, AlignSelf::Auto);
        assert_eq!(off, 0.0);
    }

    // ===== resolve driver =====

    #[test]
    fn resolve_empty_returns_zero_node() {
        let lim = limits(200.0, 100.0);
        let props: Vec<Properties> = Vec::new();
        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            |_, _| Node::default(),
        );
        assert_eq!(node.size(), Size::new(0.0, 0.0));
        assert!(node.children().is_empty());
    }

    #[test]
    fn resolve_basic_row_stretch_and_grow() {
        // Container 300x100, two items: a fixed 50x40 and a grow=1.
        // The grow item's layouter mimics a real Length::Fill widget
        // by returning a Node sized to the limits.max() — which is
        // exactly what iced's Pass 3 expects from cooperating widgets.
        let lim = limits(300.0, 100.0);

        let order: RefCell<Vec<usize>> = RefCell::new(Vec::new());
        let layouter = |idx: usize, ll: &Limits| -> Node {
            order.borrow_mut().push(idx);
            if idx == 0 {
                Node::new(Size::new(50.0, 40.0))
            } else {
                Node::new(ll.max())
            }
        };

        let mut props = vec![Properties::default(), p_grow(1.0)];
        // Item 1 is grow=1 with explicit basis 0 — so it gets the
        // entire surplus from Pass 3.
        props[1].basis = Basis::Pixels(0.0);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Fill,
            Padding::ZERO,
            0.0,
            Justify::Center,
            AlignItems::Stretch,
            false,
            &props,
            layouter,
        );

        assert_eq!(node.size(), Size::new(300.0, 100.0));
        let kids = node.children();
        // Item 0 at x=0 — Justify::Center has no leftover to
        // distribute because item 1 grew to absorb everything.
        assert_eq!(kids[0].bounds().x, 0.0);
        // Item 1 starts immediately after item 0.
        assert!((kids[1].bounds().x - 50.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_basis_pixels_skips_intrinsic() {
        // A child with Basis::Pixels(120.0) should report 120 as its
        // base size regardless of the layouter returning something
        // different. The layouter is still called once per item with
        // the assigned main as the limits' main.
        let lim = limits(400.0, 100.0);
        let fix = Fixture::new(vec![
            Size::new(120.0, 30.0), // honors limits
            Size::new(50.0, 30.0),
        ]);
        let props = vec![p_pixels(120.0), Properties::default()];

        let _node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            10.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        // The fixed-basis item still gets a layouter call (we record
        // its node), but no measurement was needed.
        let order = fix.order.borrow();
        // Each item was laid out exactly once.
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn resolve_padding_and_gap_arithmetic() {
        let lim = limits(200.0, 100.0);
        let fix =
            Fixture::new(vec![Size::new(40.0, 20.0), Size::new(40.0, 20.0)]);
        let props = props_default(2);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Shrink,
            Length::Shrink,
            Padding::new(10.0),
            5.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        // Intrinsic main = 40 + 5 + 40 = 85; with padding L+R = 20.
        // Full width = 85 + 20 = 105.
        assert!((node.size().width - 105.0).abs() < 1e-3);
        // Cross: max(20) + top+bottom padding = 20 + 20 = 40.
        assert!((node.size().height - 40.0).abs() < 1e-3);

        let kids = node.children();
        assert!((kids[0].bounds().x - 10.0).abs() < 1e-3);
        assert!((kids[1].bounds().x - 55.0).abs() < 1e-3);
        // Padding y = 10.
        assert!((kids[0].bounds().y - 10.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_reverse_with_justify_start_packs_at_visual_end() {
        // Three fixed items 50px each in a 300-wide row.
        // CSS `flex-direction: row-reverse` semantics: visual
        // main-start moves to the right edge. With Justify::Start,
        // source[0] should end up at the rightmost position
        // (x=250); source[1] at x=200; source[2] at x=150.
        let lim = limits(300.0, 50.0);
        let fix = Fixture::new(vec![
            Size::new(50.0, 20.0),
            Size::new(50.0, 20.0),
            Size::new(50.0, 20.0),
        ]);
        let props = props_default(3);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            true,
            &props,
            fix.layouter(),
        );

        let kids = node.children();
        assert!((kids[0].bounds().x - 250.0).abs() < 1e-3);
        assert!((kids[1].bounds().x - 200.0).abs() < 1e-3);
        assert!((kids[2].bounds().x - 150.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_cross_compress_with_fill_cross_uses_max_from_pass1() {
        // Row with cross-compressed (Length::Shrink height) holding:
        //   - a fixed-cross item (text-like): 60 wide, 24 tall
        //   - a fluid-cross item (button-like, height: Fill): 80 wide,
        //     intrinsic 0 tall — should adopt 24 from Pass 1's max.
        //
        // We instrument the fixture to record call order. Pass 1 must
        // call item 0 first (non-fluid-cross). Pass 2 must call item
        // 1 next, after Pass 1 has measured cross=24.

        let lim = Limits::with_compression(
            Size::ZERO,
            Size::new(400.0, 100.0),
            Size::new(false, true),
        );

        // Custom layouter that records call order *and* the cross
        // limit it received for item 1 (so we can assert it equals 24).
        let order: RefCell<Vec<usize>> = RefCell::new(Vec::new());
        let item1_max_h: RefCell<Option<f32>> = RefCell::new(None);

        let mut props = vec![
            Properties::default(),
            Properties {
                fill_cross: 1,
                ..Properties::default()
            },
        ];
        // Disable the fluid-fallback path for item 1 (we want to
        // test the cross-compress bucket, not the grow path).
        props[1].grow = 0.0;

        let layouter = |idx: usize, ll: &Limits| -> Node {
            order.borrow_mut().push(idx);
            if idx == 1 {
                *item1_max_h.borrow_mut() = Some(ll.max().height);
                Node::new(Size::new(80.0, ll.max().height))
            } else {
                Node::new(Size::new(60.0, 24.0))
            }
        };

        let _node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            layouter,
        );

        let order = order.borrow();
        assert_eq!(order[0], 0, "Pass 1 lays out item 0 first");
        assert_eq!(order[1], 1, "Pass 2 lays out item 1 next");
        assert_eq!(
            *item1_max_h.borrow(),
            Some(24.0),
            "item 1 sees cross=24 from Pass 1"
        );
    }

    #[test]
    fn resolve_infinity_main_does_not_panic() {
        // Container with infinite main: a grow item should not produce
        // NaN. Mirrors iced's flex.rs:195-205 guard.
        let lim = Limits::new(Size::ZERO, Size::INFINITE);
        let fix = Fixture::new(vec![Size::new(50.0, 20.0)]);
        let props = vec![p_grow(1.0)];

        let _node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );
        // Reaching here without panic is the assertion.
    }

    #[test]
    fn resolve_zero_grow_total_falls_back_to_fill_factor() {
        // Two FillPortion-like items with grow=0; engine should still
        // distribute remaining via fill_main_factor. Mimic a real
        // Length::Fill widget by returning Node sized to the limits.
        let lim = limits(300.0, 50.0);
        let layouter = |_idx: usize, ll: &Limits| -> Node {
            Node::new(Size::new(ll.max().width, 20.0))
        };
        let props = vec![p_fill_main(1), p_fill_main(2)];

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            layouter,
        );

        let kids = node.children();
        // Total 300 split 1:2 => item 0 takes 100, item 1 takes 200.
        assert_eq!(kids[0].bounds().x, 0.0);
        assert!((kids[1].bounds().x - 100.0).abs() < 1e-3);
        assert!((kids[1].bounds().width - 200.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_align_self_overrides_container() {
        // Two items, container align=Start; item 1 has align_self=End.
        let lim = limits(200.0, 100.0);
        let fix =
            Fixture::new(vec![Size::new(40.0, 20.0), Size::new(40.0, 30.0)]);
        let mut props = props_default(2);
        props[1].align_self = AlignSelf::End;

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Fill,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        let kids = node.children();
        assert_eq!(kids[0].bounds().y, 0.0);
        // Item 1 cross=30, container=100 => y=70.
        assert!((kids[1].bounds().y - 70.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_vertical_axis_packs_correctly() {
        // Column with two items.
        let lim = limits(100.0, 300.0);
        let fix =
            Fixture::new(vec![Size::new(40.0, 50.0), Size::new(60.0, 50.0)]);
        let props = props_default(2);

        let node = resolve(
            Axis::Vertical,
            &lim,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        let kids = node.children();
        // Vertical: items stack on y, share x=0.
        assert_eq!(kids[0].bounds().y, 0.0);
        assert_eq!(kids[1].bounds().y, 50.0);
        assert_eq!(kids[0].bounds().x, 0.0);
        assert_eq!(kids[1].bounds().x, 0.0);
        // Cross intrinsic = max(40, 60) = 60.
        assert!((node.size().width - 60.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_justify_space_between_distributes_gap() {
        let lim = limits(300.0, 50.0);
        let fix = Fixture::new(vec![
            Size::new(50.0, 20.0),
            Size::new(50.0, 20.0),
            Size::new(50.0, 20.0),
        ]);
        let props = props_default(3);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::SpaceBetween,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        let kids = node.children();
        // 150px leftover, 2 gaps => 75 each. Items at 0, 125, 250.
        assert!((kids[0].bounds().x - 0.0).abs() < 1e-3);
        assert!((kids[1].bounds().x - 125.0).abs() < 1e-3);
        assert!((kids[2].bounds().x - 250.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_justify_space_between_with_shrink_width_clamps_leftover() {
        // A `Shrink` container has no leftover to distribute, even when
        // the limit is larger than the content. Items must pack from
        // the start with no extra gap_extra — anything else would push
        // items past the container's resolved right edge.
        let lim = limits(300.0, 50.0);
        let fix = Fixture::new(vec![
            Size::new(50.0, 20.0),
            Size::new(50.0, 20.0),
            Size::new(50.0, 20.0),
        ]);
        let props = props_default(3);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Shrink, // resolves to intrinsic, not max
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::SpaceBetween,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        let kids = node.children();
        // Resolved container = intrinsic = 150. No leftover to
        // distribute. Items at 0, 50, 100.
        assert!((kids[0].bounds().x - 0.0).abs() < 1e-3);
        assert!((kids[1].bounds().x - 50.0).abs() < 1e-3);
        assert!((kids[2].bounds().x - 100.0).abs() < 1e-3);
        // Container should be intrinsic-sized, not max-sized.
        assert!((node.size().width - 150.0).abs() < 1e-3);
    }

    #[test]
    fn resolve_basis_pixels_overflow_distributes_shrink() {
        // Three items with basis=200, 300, 200 and shrink=1 in a
        // 600px container should distribute the 100px deficit weighted
        // by `basis * shrink`, per CSS:
        //   weights = [200, 300, 200], total = 700
        //   A loses 100*200/700 ≈ 28.57 → 171.43
        //   B loses 100*300/700 ≈ 42.86 → 257.14
        //   C loses 100*200/700 ≈ 28.57 → 171.43
        // Without shrink-distribution at the driver level (engine bug
        // pre-fix), items keep their basis (200/300/200 = 700) and
        // overflow the container by 100px — visible in the
        // flex-shrink demo where C bled past the bottom of the
        // canvas in column mode.
        let lim = limits(600.0, 100.0);
        let fix = Fixture::new(vec![
            Size::new(200.0, 40.0),
            Size::new(300.0, 40.0),
            Size::new(200.0, 40.0),
        ]);
        let mut props = props_default(3);
        props[0].basis = Basis::Pixels(200.0);
        props[1].basis = Basis::Pixels(300.0);
        props[2].basis = Basis::Pixels(200.0);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Fill,
            Length::Fill,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.honest_layouter(),
        );

        let kids = node.children();
        let a = kids[0].bounds().width;
        let b = kids[1].bounds().width;
        let c = kids[2].bounds().width;
        assert!((a - 171.43).abs() < 0.5, "A = {a}, expected ≈ 171.43");
        assert!((b - 257.14).abs() < 0.5, "B = {b}, expected ≈ 257.14");
        assert!((c - 171.43).abs() < 0.5, "C = {c}, expected ≈ 171.43");
        // Items must fit the container exactly — no overflow.
        assert!(
            (a + b + c - 600.0).abs() < 0.5,
            "total = {}, expected 600",
            a + b + c
        );
    }

    #[test]
    fn resolve_one_item_only() {
        let lim = limits(300.0, 50.0);
        let fix = Fixture::new(vec![Size::new(80.0, 20.0)]);
        let props = props_default(1);

        let node = resolve(
            Axis::Horizontal,
            &lim,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            5.0, // gap shouldn't matter with 1 item
            Justify::Start,
            AlignItems::Start,
            false,
            &props,
            fix.layouter(),
        );

        // Container size = item size, no gap applied (count-1 = 0).
        assert!((node.size().width - 80.0).abs() < 1e-3);
        let kids = node.children();
        assert_eq!(kids[0].bounds().x, 0.0);
    }

    #[test]
    fn resolve_pixels_basis_into_pixels() {
        // Smoke: `Pixels::from(120.0)` is what FlexChild::basis uses;
        // ensure roundtrips.
        let p = Pixels::from(120.0);
        assert_eq!(p.0, 120.0);
    }
}
