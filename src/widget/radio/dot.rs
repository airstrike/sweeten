//! Shared ring-and-dot rendering and selection-animation math for radio
//! buttons, used by both the single [`Single`](super::single::Single) and
//! the [`Group`](super::group::Group) so they animate identically.
use crate::core::border::{self, Border};
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::{Animation, Background, Color, Rectangle, Renderer};

use super::style::Style;

/// Interpolates the `off`/`on` styles at the animation's current position.
///
/// Only [`Background::Color`] is interpolable, so gradients fall back to
/// transparent; the border width and radius snap to the target while the
/// background, border, and dot colors fade in sync with the dot.
pub(super) fn blend(
    off: &Style,
    on: &Style,
    animation: &Animation<bool>,
    now: Instant,
) -> Style {
    let bg_color = |bg: Background| match bg {
        Background::Color(c) => c,
        _ => Color::TRANSPARENT,
    };

    let background = Background::Color(animation.interpolate(
        bg_color(off.background),
        bg_color(on.background),
        now,
    ));
    let border_color =
        animation.interpolate(off.border_color, on.border_color, now);
    let dot_color = animation.interpolate(off.dot_color, on.dot_color, now);

    let target = if animation.value() { *on } else { *off };

    Style {
        background,
        border_color,
        dot_color,
        ..target
    }
}

/// The `0..1` progress of the dot's fade-and-scale.
///
/// While animating this eases between the off and on positions; when idle
/// (no recorded `now`) it collapses to `1.0` if selected, else `0.0`.
pub(super) fn progress(
    animation: &Animation<bool>,
    now: Option<Instant>,
    is_selected: bool,
) -> f32 {
    match now {
        Some(now) => animation.interpolate(0.0_f32, 1.0_f32, now),
        None => {
            if is_selected {
                1.0
            } else {
                0.0
            }
        }
    }
}

/// Draws the ring into `bounds` and, when `progress > 0`, the center dot
/// scaled and faded by `progress` so it pops in rather than snapping.
pub(super) fn draw<R: Renderer>(
    renderer: &mut R,
    bounds: Rectangle,
    style: &Style,
    progress: f32,
) {
    let size = bounds.width;

    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border {
                radius: (size / 2.0).into(),
                width: style.border_width,
                color: style.border_color,
            },
            ..renderer::Quad::default()
        },
        style.background,
    );

    if progress > 0.0 {
        let scaled = (size / 2.0) * progress;
        let dot_color = Color {
            a: style.dot_color.a * progress,
            ..style.dot_color
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x + (bounds.width - scaled) / 2.0,
                    y: bounds.y + (bounds.height - scaled) / 2.0,
                    width: scaled,
                    height: scaled,
                },
                border: border::rounded(scaled / 2.0),
                ..renderer::Quad::default()
            },
            dot_color,
        );
    }
}
