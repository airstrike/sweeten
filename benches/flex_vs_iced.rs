//! Layout-throughput benchmark: sweeten's CSS-flex `Row`/`Column`
//! widgets vs. `iced_widget::Row`/`Column`.
//!
//! # What we measure
//!
//! Both sides time the *exact same call shape* —
//! `outer.as_widget_mut().layout(&mut tree, &(), &limits)`. The only
//! thing that varies between the sweeten and iced halves of a paired
//! scenario is what `outer` wraps:
//!
//! - **Sweeten**: a `sweeten::widget::flex::Row` /
//!   `sweeten::widget::flex::Column` containing `Space` children (or
//!   nested flex containers for the nested scenarios).
//! - **iced**: an `iced_widget::Row` / `iced_widget::Column` with the
//!   matching shape.
//!
//! Both Element trees go through the same per-child dyn dispatch
//! (`children[i].as_widget_mut().layout(...)`), the same `widget::Tree`
//! plumbing, and the same `Space::layout` (`layout::atomic`) leaves.
//! The only thing that differs is the *outer* flex algorithm — which
//! is what we want to measure.
//!
//! # Setup vs. timed
//!
//! Setup builds (or rebuilds) the Element + Tree per iteration since
//! `Widget::layout` takes `&mut self` and we don't want one
//! invocation's tree-state mutation to bleed into the next. Setup
//! cost is excluded from criterion's measurements — only the
//! `outer.as_widget_mut().layout(...)` call is timed.
//!
//! # Three sweeten-only scenarios (no fairness concern)
//!
//! `align_self_overrides_50`, `reverse_50`, and the
//! `micro_solve_main_sizes` micro still drive the engine through the
//! `__bench` API directly — they have no iced counterpart, so there
//! is no apples-to-apples comparison to make.
//!
//! # How to run
//!
//! ```text
//! cargo bench --bench flex_vs_iced              # full run
//! cargo bench --bench flex_vs_iced -- --quick   # ~3s/scenario smoke
//! ```

use std::hint::black_box;

use criterion::{
    BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};

use iced_core::layout::{Limits, Node};
use iced_core::widget::Tree;
use iced_core::{Alignment, Element, Length, Padding, Pixels, Size};
use iced_widget::Space;

use sweeten::widget::flex::__bench::{
    Basis, Properties, justify_offsets, resolve as sweeten_resolve,
    solve_main_sizes,
};
use sweeten::widget::flex::{self, AlignItems, Axis, Justify};

// ---------------------------------------------------------------------------
// Type aliases — both sweeten and iced widgets fit this Element shape.
// ---------------------------------------------------------------------------

/// The single `Element` type used on both sides. Generic-renderer
/// `()` works because `iced_core` provides
/// `impl Renderer for ()` (see `core/renderer/null.rs`) — no text
/// rendering is needed for `Space` children.
type Elem = Element<'static, (), iced_core::Theme, ()>;

// ---------------------------------------------------------------------------
// Helpers — Space children (used by both sides identically).
// ---------------------------------------------------------------------------

fn space_fixed(w: f32, h: f32) -> Elem {
    Space::new()
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .into()
}

fn space_fill(portion: u16) -> Elem {
    Space::new()
        .width(Length::FillPortion(portion))
        .height(Length::Fixed(20.0))
        .into()
}

// ---------------------------------------------------------------------------
// Helpers — sweeten flex widget builders.
// ---------------------------------------------------------------------------

/// Build a sweeten `flex::Row` from a child-builder closure invoked
/// `n` times. The container's only configuration is what the caller
/// passes here; defaults match `flex::Row::new()`.
#[allow(clippy::too_many_arguments)]
fn sw_row<F>(
    n: usize,
    width: Length,
    height: Length,
    padding: Padding,
    spacing: f32,
    justify: Justify,
    align: AlignItems,
    reverse: bool,
    mut child: F,
) -> Elem
where
    F: FnMut(usize) -> Elem,
{
    let mut row = flex::Row::new()
        .width(width)
        .height(height)
        .padding(padding)
        .spacing(spacing)
        .justify(justify)
        .align(align)
        .reverse(reverse);
    for i in 0..n {
        row = row.push(child(i));
    }
    row.into()
}

#[allow(clippy::too_many_arguments)]
fn sw_column<F>(
    n: usize,
    width: Length,
    height: Length,
    padding: Padding,
    spacing: f32,
    justify: Justify,
    align: AlignItems,
    reverse: bool,
    mut child: F,
) -> Elem
where
    F: FnMut(usize) -> Elem,
{
    let mut col = flex::Column::new()
        .width(width)
        .height(height)
        .padding(padding)
        .spacing(spacing)
        .justify(justify)
        .align(align)
        .reverse(reverse);
    for i in 0..n {
        col = col.push(child(i));
    }
    col.into()
}

// ---------------------------------------------------------------------------
// Helpers — iced widget::Row/Column builders (mirror shape of sw_*).
// ---------------------------------------------------------------------------

fn ic_row<F>(
    n: usize,
    width: Length,
    height: Length,
    padding: Padding,
    spacing: f32,
    align: Alignment,
    mut child: F,
) -> Elem
where
    F: FnMut(usize) -> Elem,
{
    let mut row = iced_widget::Row::<(), iced_core::Theme, ()>::new()
        .width(width)
        .height(height)
        .padding(padding)
        .spacing(Pixels(spacing))
        .align_y(align);
    for i in 0..n {
        row = row.push(child(i));
    }
    row.into()
}

fn ic_column<F>(
    n: usize,
    width: Length,
    height: Length,
    padding: Padding,
    spacing: f32,
    align: Alignment,
    mut child: F,
) -> Elem
where
    F: FnMut(usize) -> Elem,
{
    let mut col = iced_widget::Column::<(), iced_core::Theme, ()>::new()
        .width(width)
        .height(height)
        .padding(padding)
        .spacing(Pixels(spacing))
        .align_x(align);
    for i in 0..n {
        col = col.push(child(i));
    }
    col.into()
}

// ---------------------------------------------------------------------------
// Helpers — engine-only sweeten driver (used by sweeten-only scenarios).
// ---------------------------------------------------------------------------

/// CSS-default `Properties` (grow=0, shrink=1, basis=Auto).
fn p_default() -> Properties {
    Properties {
        grow: 0.0,
        shrink: 1.0,
        basis: Basis::Auto,
        align_self: Default::default(),
        fill_main: 0,
        fill_cross: 0,
    }
}

/// A pre-classified fluid item: `grow=factor, basis=Pixels(0)`.
fn p_fluid(factor: f32) -> Properties {
    Properties {
        grow: factor,
        shrink: 1.0,
        basis: Basis::Pixels(0.0),
        align_self: Default::default(),
        fill_main: factor as u16,
        fill_cross: 0,
    }
}

/// A constant-size layouter — mirrors `Space::layout` under
/// `layout::atomic`: each child reports the size it was asked for,
/// clamped to the limits.
fn fixed_layouter(w: f32, h: f32) -> impl FnMut(usize, &Limits) -> Node {
    move |_idx, limits| {
        let max = limits.max();
        Node::new(Size::new(w.min(max.width), h.min(max.height)))
    }
}

// ---------------------------------------------------------------------------
// Common driver — time the same call on both sides.
// ---------------------------------------------------------------------------

/// Lay out an [`Elem`] by building its [`Tree`] and calling
/// `outer.as_widget_mut().layout(...)`. The Tree must be freshly built
/// (in the criterion setup closure) so each timed iteration starts
/// from the same state.
fn layout_outer(outer: &mut Elem, limits: &Limits) -> Node {
    let mut tree = Tree::new(outer.as_widget());
    outer.as_widget_mut().layout(&mut tree, &(), limits)
}

// ---------------------------------------------------------------------------
// Scenario 1 — Throughput (plain fixed-size children, varying N).
// ---------------------------------------------------------------------------

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_row_fixed");
    let limits = Limits::new(Size::ZERO, Size::new(10_000.0, 200.0));

    for &n in &[10usize, 100, 1000] {
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("sweeten", n), &n, |b, _n| {
            b.iter_with_setup(
                || {
                    sw_row(
                        n,
                        Length::Shrink,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        Justify::Start,
                        AlignItems::Start,
                        false,
                        |_| space_fixed(40.0, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });

        group.bench_with_input(BenchmarkId::new("iced", n), &n, |b, _n| {
            b.iter_with_setup(
                || {
                    ic_row(
                        n,
                        Length::Shrink,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        Alignment::Start,
                        |_| space_fixed(40.0, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 2 — Grow / Fill distribution.
// ---------------------------------------------------------------------------

fn bench_grow(c: &mut Criterion) {
    let mut group = c.benchmark_group("grow_three_children");
    let limits = Limits::new(Size::ZERO, Size::new(1200.0, 200.0));

    // Three Length::FillPortion children — equivalent on both sides
    // since sweeten's FlexChild::resolved_properties translates
    // Length::Fill / Length::FillPortion(n) to (grow=n, basis=0).
    let portions = [1u16, 2, 1];

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    portions.len(),
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    |i| space_fill(portions[i]),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    portions.len(),
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Alignment::Start,
                    |i| space_fill(portions[i]),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 3 — Shrink under deficit.
// ---------------------------------------------------------------------------
//
// Sweeten runs the CSS shrink algorithm (basis * shrink) and re-lays
// out shrunk items in Pass S. iced has no equivalent — its row simply
// overflows. So the comparison is "performance cost of a feature iced
// doesn't have" rather than apples-to-apples; both sides still go
// through identical Element/Tree machinery.

fn bench_shrink(c: &mut Criterion) {
    let mut group = c.benchmark_group("shrink_under_deficit");
    let limits = Limits::new(Size::ZERO, Size::new(600.0, 200.0));

    for &n in &[3usize, 30, 300] {
        group.throughput(Throughput::Elements(n as u64));
        // Per-item width such that total = 1.5x the budget.
        let w = 600.0 * 1.5 / n as f32;

        group.bench_with_input(BenchmarkId::new("sweeten", n), &n, |b, _n| {
            b.iter_with_setup(
                || {
                    sw_row(
                        n,
                        Length::Fill,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        Justify::Start,
                        AlignItems::Start,
                        false,
                        |_| space_fixed(w, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });

        group.bench_with_input(BenchmarkId::new("iced", n), &n, |b, _n| {
            b.iter_with_setup(
                || {
                    ic_row(
                        n,
                        Length::Fill,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        Alignment::Start,
                        |_| space_fixed(w, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 4 — Single-level nested (row of columns of children).
// ---------------------------------------------------------------------------

fn bench_nested_one_level(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_one_level_5x5");
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 1000.0));
    const OUTER: usize = 5;
    const INNER: usize = 5;

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    OUTER,
                    Length::Shrink,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    |_| {
                        sw_column(
                            INNER,
                            Length::Shrink,
                            Length::Shrink,
                            Padding::ZERO,
                            0.0,
                            Justify::Start,
                            AlignItems::Start,
                            false,
                            |_| space_fixed(40.0, 20.0),
                        )
                    },
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    OUTER,
                    Length::Shrink,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Alignment::Start,
                    |_| {
                        ic_column(
                            INNER,
                            Length::Shrink,
                            Length::Shrink,
                            Padding::ZERO,
                            0.0,
                            Alignment::Start,
                            |_| space_fixed(40.0, 20.0),
                        )
                    },
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 5 — Deeply nested (alternating rows/columns, 8 / 10 levels).
// ---------------------------------------------------------------------------
//
// Each level holds `fanout` children; the innermost level holds
// fanout fixed spaces. Recursion cost dominates over per-child cost.

fn build_sweeten_nested(depth: usize, fanout: usize, axis: Axis) -> Elem {
    if depth == 0 {
        return space_fixed(20.0, 20.0);
    }
    let next_axis = match axis {
        Axis::Horizontal => Axis::Vertical,
        Axis::Vertical => Axis::Horizontal,
    };
    match axis {
        Axis::Horizontal => sw_row(
            fanout,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            |_| build_sweeten_nested(depth - 1, fanout, next_axis),
        ),
        Axis::Vertical => sw_column(
            fanout,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Justify::Start,
            AlignItems::Start,
            false,
            |_| build_sweeten_nested(depth - 1, fanout, next_axis),
        ),
    }
}

/// Iced's analogue of `Axis` for the nested builder — internal-only.
#[derive(Clone, Copy)]
enum IcAxis {
    Row,
    Column,
}

fn build_iced_nested(depth: usize, fanout: usize, axis: IcAxis) -> Elem {
    if depth == 0 {
        return space_fixed(20.0, 20.0);
    }
    let next_axis = match axis {
        IcAxis::Row => IcAxis::Column,
        IcAxis::Column => IcAxis::Row,
    };
    match axis {
        IcAxis::Row => ic_row(
            fanout,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Alignment::Start,
            |_| build_iced_nested(depth - 1, fanout, next_axis),
        ),
        IcAxis::Column => ic_column(
            fanout,
            Length::Shrink,
            Length::Shrink,
            Padding::ZERO,
            0.0,
            Alignment::Start,
            |_| build_iced_nested(depth - 1, fanout, next_axis),
        ),
    }
}

fn bench_deeply_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("deeply_nested");
    let limits = Limits::new(Size::ZERO, Size::new(10_000.0, 10_000.0));

    for &(depth, fanout) in &[(8usize, 3usize), (10, 3)] {
        let label = format!("d{depth}_f{fanout}");

        group.bench_function(BenchmarkId::new("sweeten", &label), |b| {
            b.iter_with_setup(
                || build_sweeten_nested(depth, fanout, Axis::Horizontal),
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });

        group.bench_function(BenchmarkId::new("iced", &label), |b| {
            b.iter_with_setup(
                || build_iced_nested(depth, fanout, IcAxis::Row),
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 6 — Heterogeneous mix (some grow, some fixed).
// ---------------------------------------------------------------------------

fn bench_heterogeneous(c: &mut Criterion) {
    let mut group = c.benchmark_group("heterogeneous_50_children");
    const N: usize = 50;
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 200.0));

    let make_child = |i: usize| -> Elem {
        match i % 3 {
            0 => space_fixed(40.0, 20.0),
            1 => space_fill(1),
            _ => space_fixed(80.0, 20.0),
        }
    };

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    4.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    make_child,
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    4.0,
                    Alignment::Start,
                    make_child,
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 7 — Wide-bounds best case (100 fixed children fit easily).
// ---------------------------------------------------------------------------

fn bench_wide_bounds(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_bounds_100");
    const N: usize = 100;
    let limits = Limits::new(Size::ZERO, Size::new(10_000.0, 200.0));

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    |_| space_fixed(40.0, 20.0),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Alignment::Start,
                    |_| space_fixed(40.0, 20.0),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 8 — Tight bounds (every child needs shrinking).
// ---------------------------------------------------------------------------

fn bench_tight_bounds(c: &mut Criterion) {
    let mut group = c.benchmark_group("tight_bounds_100");
    const N: usize = 100;
    let limits = Limits::new(Size::ZERO, Size::new(500.0, 200.0));

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    |_| space_fixed(20.0, 20.0),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Alignment::Start,
                    |_| space_fixed(20.0, 20.0),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 9 — Padding + gap with grow.
// ---------------------------------------------------------------------------

fn bench_padding_gap(c: &mut Criterion) {
    let mut group = c.benchmark_group("padding_gap_grow_20");
    const N: usize = 20;
    let limits = Limits::new(Size::ZERO, Size::new(1500.0, 200.0));

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::new(16.0),
                    8.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    |_| space_fill(1),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::new(16.0),
                    8.0,
                    Alignment::Start,
                    |_| space_fill(1),
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 10 — align_items variants (Start / Center / End / Stretch).
// ---------------------------------------------------------------------------
//
// `iced::Alignment` has no `Stretch`, so the iced side of the Stretch
// case is omitted — sweeten-only by structural necessity.

fn bench_align_items(c: &mut Criterion) {
    let mut group = c.benchmark_group("align_items_50");
    const N: usize = 50;
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 200.0));

    for (label, sw_align, ic_align) in [
        ("start", AlignItems::Start, Some(Alignment::Start)),
        ("center", AlignItems::Center, Some(Alignment::Center)),
        ("end", AlignItems::End, Some(Alignment::End)),
        ("stretch", AlignItems::Stretch, None), // no iced equivalent
    ] {
        group.bench_function(BenchmarkId::new("sweeten", label), |b| {
            b.iter_with_setup(
                || {
                    sw_row(
                        N,
                        Length::Fill,
                        Length::Fill,
                        Padding::ZERO,
                        0.0,
                        Justify::Start,
                        sw_align,
                        false,
                        |_| space_fixed(40.0, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });

        if let Some(ic) = ic_align {
            group.bench_function(BenchmarkId::new("iced", label), |b| {
                b.iter_with_setup(
                    || {
                        ic_row(
                            N,
                            Length::Fill,
                            Length::Fill,
                            Padding::ZERO,
                            0.0,
                            ic,
                            |_| space_fixed(40.0, 20.0),
                        )
                    },
                    |mut outer| {
                        let node = layout_outer(&mut outer, &limits);
                        black_box(node);
                    },
                );
            });
        }
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 11 — justify_content variants (Start/End/Center + Space*).
// ---------------------------------------------------------------------------
//
// iced's row has no `justify-content`, but the same visual outcomes
// can be approximated by inserting `Space` widgets with
// `Length::FillPortion` between/around the real children. The shapes
// used here:
//
//   Start:        [c × N]
//   End:          F(1) [c × N]
//   Center:       F(1) [c × N] F(1)
//   SpaceBetween: c F(1) c F(1) … c           (N children, N-1 fills)
//   SpaceAround:  F(1) c F(2) c … c F(1)      (N children, N+1 fills)
//   SpaceEvenly:  F(1) c F(1) c … c F(1)      (N children, N+1 fills)
//
// The iced side lays out more child elements (children + spacers),
// while sweeten does the same N children with extra justify math per
// child. The benches measure user-equivalent code, not strictly
// identical algorithmic work — that is the point.

fn iced_justify_elems(justify: Justify, n: usize) -> Vec<Elem> {
    let child = || space_fixed(40.0, 20.0);
    match justify {
        Justify::Start => (0..n).map(|_| child()).collect(),
        Justify::End => {
            let mut v = Vec::with_capacity(n + 1);
            v.push(space_fill(1));
            v.extend((0..n).map(|_| child()));
            v
        }
        Justify::Center => {
            let mut v = Vec::with_capacity(n + 2);
            v.push(space_fill(1));
            v.extend((0..n).map(|_| child()));
            v.push(space_fill(1));
            v
        }
        Justify::SpaceBetween => {
            let mut v = Vec::with_capacity(2 * n - 1);
            for i in 0..n {
                if i > 0 {
                    v.push(space_fill(1));
                }
                v.push(child());
            }
            v
        }
        Justify::SpaceAround => {
            let mut v = Vec::with_capacity(2 * n + 1);
            v.push(space_fill(1));
            for i in 0..n {
                if i > 0 {
                    v.push(space_fill(2));
                }
                v.push(child());
            }
            v.push(space_fill(1));
            v
        }
        Justify::SpaceEvenly => {
            let mut v = Vec::with_capacity(2 * n + 1);
            v.push(space_fill(1));
            for _ in 0..n {
                v.push(child());
                v.push(space_fill(1));
            }
            v
        }
    }
}

fn bench_justify_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("justify_content_20");
    const N: usize = 20;
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 200.0));

    for (label, justify) in [
        ("start", Justify::Start),
        ("center", Justify::Center),
        ("end", Justify::End),
        ("between", Justify::SpaceBetween),
        ("around", Justify::SpaceAround),
        ("evenly", Justify::SpaceEvenly),
    ] {
        group.bench_function(BenchmarkId::new("sweeten", label), |b| {
            b.iter_with_setup(
                || {
                    sw_row(
                        N,
                        Length::Fill,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        justify,
                        AlignItems::Start,
                        false,
                        |_| space_fixed(40.0, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });

        group.bench_function(BenchmarkId::new("iced", label), |b| {
            b.iter_with_setup(
                || {
                    let elems = iced_justify_elems(justify, N);
                    let outer: Elem = iced_widget::Row::<
                        (),
                        iced_core::Theme,
                        (),
                    >::with_children(
                        elems
                    )
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .padding(Padding::ZERO)
                    .spacing(Pixels(0.0))
                    .align_y(Alignment::Start)
                    .into();
                    outer
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 12 — align_self overrides on a few children (sweeten-only).
// ---------------------------------------------------------------------------

fn bench_align_self(c: &mut Criterion) {
    let mut group = c.benchmark_group("align_self_overrides_50");
    const N: usize = 50;
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 200.0));

    use sweeten::widget::flex::AlignSelf;
    let props: Vec<Properties> = (0..N)
        .map(|i| {
            let mut p = p_default();
            // Every 5th child overrides cross alignment.
            if i % 5 == 0 {
                p.align_self = AlignSelf::End;
            }
            p
        })
        .collect();

    group.bench_function("sweeten", |b| {
        b.iter(|| {
            let node = sweeten_resolve(
                Axis::Horizontal,
                &limits,
                Length::Fill,
                Length::Fill,
                Padding::ZERO,
                0.0,
                Justify::Start,
                AlignItems::Start,
                false,
                &props,
                fixed_layouter(40.0, 20.0),
            );
            black_box(node);
        });
    });
    // No iced equivalent for align-self overrides.
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 13 — Reverse direction (sweeten-only — iced has no
// equivalent on the resolver itself).
// ---------------------------------------------------------------------------

fn bench_reverse(c: &mut Criterion) {
    let mut group = c.benchmark_group("reverse_50");
    const N: usize = 50;
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 200.0));
    let props: Vec<Properties> = (0..N).map(|_| p_default()).collect();

    group.bench_function("forward", |b| {
        b.iter(|| {
            let node = sweeten_resolve(
                Axis::Horizontal,
                &limits,
                Length::Fill,
                Length::Shrink,
                Padding::ZERO,
                0.0,
                Justify::Start,
                AlignItems::Start,
                false,
                &props,
                fixed_layouter(40.0, 20.0),
            );
            black_box(node);
        });
    });
    group.bench_function("reverse", |b| {
        b.iter(|| {
            let node = sweeten_resolve(
                Axis::Horizontal,
                &limits,
                Length::Fill,
                Length::Shrink,
                Padding::ZERO,
                0.0,
                Justify::Start,
                AlignItems::Start,
                true,
                &props,
                fixed_layouter(40.0, 20.0),
            );
            black_box(node);
        });
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 14 — Pixel-basis mixed with growers (`Demo::Basis` shape).
// ---------------------------------------------------------------------------

fn bench_basis_mix(c: &mut Criterion) {
    let mut group = c.benchmark_group("basis_mix_20");
    const N: usize = 20;
    let limits = Limits::new(Size::ZERO, Size::new(1500.0, 200.0));

    let make_child = |i: usize| -> Elem {
        if i.is_multiple_of(2) {
            space_fixed(60.0, 20.0)
        } else {
            space_fill(1)
        }
    };

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                sw_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    make_child,
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                ic_row(
                    N,
                    Length::Fill,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Alignment::Start,
                    make_child,
                )
            },
            |mut outer| {
                let node = layout_outer(&mut outer, &limits);
                black_box(node);
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 15 — Repeated layout amortization (same tree N times).
// ---------------------------------------------------------------------------
//
// Neither resolver caches between calls — there is no widget-tree
// state that survives across `resolve` invocations on this code path.
// We bench 100 layouts of the same 50-child container to confirm the
// per-call cost is constant (vs. some hidden warmup).

fn bench_repeated(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeated_layouts_x100");
    const N: usize = 50;
    const REPS: usize = 100;
    let limits = Limits::new(Size::ZERO, Size::new(2000.0, 200.0));

    group.throughput(Throughput::Elements(REPS as u64));

    group.bench_function("sweeten", |b| {
        b.iter_with_setup(
            || {
                let outer = sw_row(
                    N,
                    Length::Shrink,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Justify::Start,
                    AlignItems::Start,
                    false,
                    |_| space_fixed(40.0, 20.0),
                );
                let tree = Tree::new(outer.as_widget());
                (outer, tree)
            },
            |(mut outer, mut tree)| {
                for _ in 0..REPS {
                    let node =
                        outer.as_widget_mut().layout(&mut tree, &(), &limits);
                    black_box(node);
                }
            },
        );
    });

    group.bench_function("iced", |b| {
        b.iter_with_setup(
            || {
                let outer = ic_row(
                    N,
                    Length::Shrink,
                    Length::Shrink,
                    Padding::ZERO,
                    0.0,
                    Alignment::Start,
                    |_| space_fixed(40.0, 20.0),
                );
                let tree = Tree::new(outer.as_widget());
                (outer, tree)
            },
            |(mut outer, mut tree)| {
                for _ in 0..REPS {
                    let node =
                        outer.as_widget_mut().layout(&mut tree, &(), &limits);
                    black_box(node);
                }
            },
        );
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 16 — Vertical (Column) throughput.
// ---------------------------------------------------------------------------

fn bench_vertical_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertical_column_fixed");
    let limits = Limits::new(Size::ZERO, Size::new(200.0, 10_000.0));

    for &n in &[100usize, 1000] {
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::new("sweeten", n), &n, |b, _n| {
            b.iter_with_setup(
                || {
                    sw_column(
                        n,
                        Length::Shrink,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        Justify::Start,
                        AlignItems::Start,
                        false,
                        |_| space_fixed(40.0, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });

        group.bench_with_input(BenchmarkId::new("iced", n), &n, |b, _n| {
            b.iter_with_setup(
                || {
                    ic_column(
                        n,
                        Length::Shrink,
                        Length::Shrink,
                        Padding::ZERO,
                        0.0,
                        Alignment::Start,
                        |_| space_fixed(40.0, 20.0),
                    )
                },
                |mut outer| {
                    let node = layout_outer(&mut outer, &limits);
                    black_box(node);
                },
            );
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Scenario 17 — solve_main_sizes microbenchmark (sweeten-internal).
// ---------------------------------------------------------------------------

fn bench_solve_microbench(c: &mut Criterion) {
    let mut group = c.benchmark_group("micro_solve_main_sizes");
    for &n in &[10usize, 100, 1000] {
        let base: Vec<f32> = (0..n).map(|i| 20.0 + i as f32).collect();
        let props: Vec<Properties> = (0..n)
            .map(|i| {
                if i % 2 == 0 {
                    p_fluid(1.0)
                } else {
                    p_default()
                }
            })
            .collect();
        group.bench_with_input(
            BenchmarkId::new("solve_main_sizes", n),
            &n,
            |b, _| {
                b.iter(|| black_box(solve_main_sizes(&base, &props, 500.0)));
            },
        );
    }
    group.bench_function("justify_offsets", |b| {
        b.iter(|| {
            black_box(justify_offsets(
                black_box(50),
                black_box(120.0),
                black_box(Justify::SpaceBetween),
            ))
        });
    });
    group.finish();
}

// ---------------------------------------------------------------------------
// Wiring.
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_throughput,
    bench_grow,
    bench_shrink,
    bench_nested_one_level,
    bench_deeply_nested,
    bench_heterogeneous,
    bench_wide_bounds,
    bench_tight_bounds,
    bench_padding_gap,
    bench_align_items,
    bench_justify_content,
    bench_align_self,
    bench_reverse,
    bench_basis_mix,
    bench_repeated,
    bench_vertical_throughput,
    bench_solve_microbench,
);
criterion_main!(benches);
