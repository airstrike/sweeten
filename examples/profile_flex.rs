//! Profile target for the sweeten flex engine.
//!
//! Builds either a flat 1000-child row or a depth-10 fanout-3 nested
//! tree, then calls `Widget::layout()` in a hot loop for a fixed wall
//! clock duration. Intended to be driven by `samply record --`.
//!
//! Usage:
//!   cargo build --release --example profile_flex
//!   samply record -- ./target/release/examples/profile_flex --flat
//!   samply record -- ./target/release/examples/profile_flex --deep
//!
//! Both flavors rebuild the Element + Tree per iteration to mirror the
//! `flex_vs_iced` bench (Widget::layout takes &mut self; we don't want
//! the tree-state mutation from one call to bleed into the next).

use std::hint::black_box;
use std::time::{Duration, Instant};

use iced_core::layout::Limits;
use iced_core::widget::Tree;
use iced_core::{Element, Length, Padding, Size};
use iced_widget::Space;

use sweeten::widget::flex::{self, AlignItems, Axis, Justify};

type Elem = Element<'static, (), iced_core::Theme, ()>;

fn space_fixed(w: f32, h: f32) -> Elem {
    Space::new()
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .into()
}

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

fn build_flat() -> Elem {
    sw_row(
        1000,
        Length::Shrink,
        Length::Shrink,
        Padding::ZERO,
        0.0,
        Justify::Start,
        AlignItems::Start,
        false,
        |_| space_fixed(40.0, 20.0),
    )
}

fn build_deep_nested(depth: usize, fanout: usize, axis: Axis) -> Elem {
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
            |_| build_deep_nested(depth - 1, fanout, next_axis),
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
            |_| build_deep_nested(depth - 1, fanout, next_axis),
        ),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.iter().skip(1).find_map(|a| match a.as_str() {
        "--flat" => Some("flat"),
        "--deep" => Some("deep"),
        _ => None,
    });
    let mode = match mode {
        Some(m) => m,
        None => {
            eprintln!("usage: profile_flex --flat | --deep");
            std::process::exit(2);
        }
    };

    // Total wall-clock budget per profile run. samply records the whole
    // process, so this also bounds the sample window.
    let budget = Duration::from_secs(10);
    let warmup = Duration::from_millis(500);

    // For the flat scenario we use wide bounds so children fit; for the
    // deep nested scenario we use the same limits as `bench_deeply_nested`.
    let limits = match mode {
        "flat" => Limits::new(Size::ZERO, Size::new(100_000.0, 200.0)),
        "deep" => Limits::new(Size::ZERO, Size::new(10_000.0, 10_000.0)),
        _ => unreachable!(),
    };

    eprintln!("profile_flex: mode={mode} budget={budget:?}");

    // Warmup so the first few iterations (cold allocator / icache) don't
    // dominate the trace.
    let warmup_start = Instant::now();
    let mut warmup_iters: u64 = 0;
    while warmup_start.elapsed() < warmup {
        let mut outer = match mode {
            "flat" => build_flat(),
            "deep" => build_deep_nested(10, 3, Axis::Horizontal),
            _ => unreachable!(),
        };
        let mut tree = Tree::new(outer.as_widget());
        let node = outer.as_widget_mut().layout(&mut tree, &(), &limits);
        black_box(node);
        warmup_iters += 1;
    }
    eprintln!("profile_flex: warmup_iters={warmup_iters}");

    let start = Instant::now();
    let mut iters: u64 = 0;
    while start.elapsed() < budget {
        // Rebuild per iteration — same as the criterion bench's
        // iter_with_setup. The build cost is in the trace but is
        // independent of `resolve`, and samply will separate them by
        // function.
        let mut outer = match mode {
            "flat" => build_flat(),
            "deep" => build_deep_nested(10, 3, Axis::Horizontal),
            _ => unreachable!(),
        };
        let mut tree = Tree::new(outer.as_widget());
        let node = outer.as_widget_mut().layout(&mut tree, &(), &limits);
        black_box(node);
        iters += 1;
    }
    let elapsed = start.elapsed();
    eprintln!(
        "profile_flex: iters={iters} elapsed={elapsed:?} \
         per_iter={:?}",
        elapsed / iters.max(1) as u32,
    );
}
