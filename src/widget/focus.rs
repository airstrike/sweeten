//! Focus utilities shared across sweetened widgets.
//!
//! [`Source`] records *how* a widget most recently gained focus, so a
//! widget can show its focus ring only when navigated by keyboard — the
//! analog of CSS `:focus-visible` — and not when clicked.
//!
//! The upstream [`Focusable`] trait these widgets implement is re-exported
//! here so the focus concepts live together; see the
//! [`operation`](crate::widget::operation) module for the `focus_next` /
//! `focus_previous` helpers that drive it.

use crate::core::border::Border;
use crate::core::renderer;
use crate::core::{Background, Color, Rectangle, Renderer};

pub use crate::core::widget::operation::Focusable;

/// How a focusable widget most recently gained focus.
///
/// Widgets that draw a focus ring only under keyboard navigation store
/// this behind an [`Option`] — `None` meaning "not focused" — and paint
/// the ring only for [`Keyboard`](Self::Keyboard), the analog of CSS
/// `:focus-visible`. Clicking a widget focuses it for subsequent keyboard
/// use but records [`Mouse`](Self::Mouse), so no ring is shown until the
/// next keyboard interaction re-arms [`Keyboard`](Self::Keyboard).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Focus arrived via Tab, a programmatic focus operation, or keyboard
    /// navigation / activation.
    Keyboard,
    /// Focus arrived by clicking or tapping the widget.
    Mouse,
}

/// Draws a soft `:focus-visible` halo hugging a control.
///
/// `bounds` and `radius` are the control's own bounds and corner radius
/// (pass `height / 2.0` for a circle like a radio dot, or the box radius
/// for a checkbox). The band is expanded outward by a small gap and kept
/// concentric with the control — `radius + GAP` — so its corners parallel
/// the control's. It is thin and drawn at reduced alpha so it reads as a
/// glow rather than a hard outline.
pub(crate) fn ring<R: Renderer>(
    renderer: &mut R,
    bounds: Rectangle,
    radius: f32,
    color: Color,
) {
    /// Gap between the control's edge and the ring band.
    const GAP: f32 = 3.0;

    renderer.fill_quad(
        renderer::Quad {
            bounds: bounds.expand(GAP),
            border: Border {
                radius: (radius + GAP).into(),
                width: 2.0,
                color: color.scale_alpha(0.4),
            },
            ..renderer::Quad::default()
        },
        Background::Color(Color::TRANSPARENT),
    );
}
