//! [`FlexChild`] — an [`Element`] paired with CSS-flex properties.
//!
//! Users opt into per-item flex behavior by wrapping an element in a
//! [`FlexChild`] (typically through the `flex(elem)` helper added in a
//! later phase) and chaining builder methods. Plain elements are
//! accepted via the `From<E: Into<Element>>` blanket impl, so they
//! enter the flex container with CSS-default properties.

use crate::core::{Element, Pixels};

use super::alignment::{AlignSelf, Axis};

/// Per-child flex properties resolved by the engine.
///
/// `Properties` is `pub(crate)` because users never construct it
/// directly — they build it through [`FlexChild`] builder methods. The
/// engine takes a `&[Properties]` slice when laying items out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Properties {
    /// CSS `flex-grow` — non-negative scale factor for distributing
    /// surplus main-axis space.
    pub grow: f32,
    /// CSS `flex-shrink` — non-negative scale factor for distributing
    /// main-axis deficit. CSS shrink scales by `shrink * basis` so
    /// larger items absorb more of the deficit.
    pub shrink: f32,
    /// CSS `flex-basis` — the item's initial main size before grow /
    /// shrink distribution.
    pub basis: Basis,
    /// Per-item cross-axis alignment override.
    pub align_self: AlignSelf,
    /// Fallback main-axis fill factor derived from the inner widget's
    /// `Length` hint. Used when `grow == 0` to preserve iced's
    /// `Length::FillPortion(n)` semantics for items that have not opted
    /// in to explicit grow.
    pub fill_main: u16,
    /// Fallback cross-axis fill factor derived from the inner widget's
    /// `Length` hint. Drives the cross-compress deferred bucket exactly
    /// the way iced's resolver does.
    pub fill_cross: u16,
}

impl Default for Properties {
    fn default() -> Self {
        Self {
            grow: 0.0,
            shrink: 1.0,
            basis: Basis::Auto,
            align_self: AlignSelf::Auto,
            fill_main: 0,
            fill_cross: 0,
        }
    }
}

/// CSS `flex-basis` value.
///
/// `Auto` defers to the inner widget's intrinsic main size (Pass 1 of
/// the engine measures it). `Pixels(p)` forces a specific basis and
/// skips Pass 1 for that item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Basis {
    /// Use the widget's intrinsic main size as the basis.
    Auto,
    /// Force a specific main-size basis in pixels.
    Pixels(f32),
}

/// An [`Element`] enriched with CSS-flex properties.
///
/// Construct via the `flex(elem)` helper (added in a later phase) or
/// implicitly via `From<E: Into<Element>>` — passing a plain element
/// to `flex_row![...]` wraps it as a `FlexChild` with CSS defaults
/// (`grow=0`, `shrink=1`, `basis=auto`, `align_self=auto`).
#[allow(missing_debug_implementations)]
pub struct FlexChild<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> {
    content: Element<'a, Message, Theme, Renderer>,
    properties: Properties,
}

impl<'a, Message, Theme, Renderer> FlexChild<'a, Message, Theme, Renderer> {
    /// Creates a new [`FlexChild`] wrapping `content` with CSS-default
    /// properties.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            content: content.into(),
            properties: Properties::default(),
        }
    }

    /// Sets the CSS `flex-grow` factor.
    ///
    /// Negative values are clamped to zero — CSS specifies non-negative
    /// grow factors.
    pub fn grow(mut self, grow: f32) -> Self {
        self.properties.grow = grow.max(0.0);
        self
    }

    /// Sets the CSS `flex-shrink` factor.
    ///
    /// Negative values are clamped to zero — CSS specifies non-negative
    /// shrink factors.
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.properties.shrink = shrink.max(0.0);
        self
    }

    /// Sets the CSS `flex-basis` to a fixed pixel value.
    ///
    /// Skips the basis-measurement pass for this child — the engine
    /// uses the supplied value directly as the initial main size.
    pub fn basis(mut self, basis: impl Into<Pixels>) -> Self {
        let Pixels(p) = basis.into();
        self.properties.basis = Basis::Pixels(p);
        self
    }

    /// Sets the per-item cross-axis alignment, overriding the
    /// container's [`super::AlignItems`].
    pub fn align_self(mut self, align: impl Into<AlignSelf>) -> Self {
        self.properties.align_self = align.into();
        self
    }

    /// Returns a reference to the wrapped [`Element`].
    pub fn content(&self) -> &Element<'a, Message, Theme, Renderer> {
        &self.content
    }

    /// Returns a mutable reference to the wrapped [`Element`].
    pub fn content_mut(
        &mut self,
    ) -> &mut Element<'a, Message, Theme, Renderer> {
        &mut self.content
    }

    /// Consumes the [`FlexChild`] and returns the wrapped [`Element`].
    pub fn into_content(self) -> Element<'a, Message, Theme, Renderer> {
        self.content
    }

    /// Returns the engine-facing properties for this child, with
    /// fluid-fallback factors derived from the inner widget's `Length`
    /// hint along `axis`.
    ///
    /// Items whose main length is `Length::Fill`/`FillPortion(_)` and
    /// who haven't opted in to a custom basis enter as
    /// `(grow=fill_factor, basis=Pixels(0))`, matching iced's existing
    /// behavior of giving fluid items a zero base size and dividing
    /// the remaining space by fill factor. This preserves the common
    /// case's performance characteristics — a single layout pass per
    /// fluid child.
    pub(crate) fn resolved_properties(&self, axis: Axis) -> Properties
    where
        Renderer: crate::core::Renderer,
    {
        let size = self.content.as_widget().size();
        let (main_len, cross_len) = axis.pack(size.width, size.height);

        let fill_main = main_len.fill_factor();
        let fill_cross = cross_len.fill_factor();

        let mut props = self.properties;
        props.fill_main = fill_main;
        props.fill_cross = fill_cross;

        // Explicit Basis::Pixels means the user has stated the
        // main-axis size. The inner widget's Length::Fill hint is
        // consulted only as a basis-Auto fallback below; a pixel basis
        // overrides it and pins the item to the fixed-main bucket.
        // Without this, an item like `flex(boxed).basis(80)` whose
        // inner widget has Length::Fill width drops into Pass 3's
        // fluid bucket, gets share_main=0 alongside a sibling with
        // grow=1, and renders at zero width.
        if matches!(props.basis, Basis::Pixels(_)) {
            props.fill_main = 0;
        }

        // Fluid-fallback: a Length::Fill item with no explicit grow and
        // an Auto basis behaves as `(grow=fill_factor, basis=0)` so the
        // engine's grow-distribution path handles it in one layout
        // pass — same shape as iced's Pass 3 today.
        if props.grow == 0.0
            && fill_main > 0
            && matches!(props.basis, Basis::Auto)
        {
            props.grow = f32::from(fill_main);
            props.basis = Basis::Pixels(0.0);
        }

        props
    }
}

impl<'a, E, Message, Theme, Renderer> From<E>
    for FlexChild<'a, Message, Theme, Renderer>
where
    E: Into<Element<'a, Message, Theme, Renderer>>,
{
    fn from(content: E) -> Self {
        FlexChild::new(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_css() {
        let p = Properties::default();
        assert_eq!(p.grow, 0.0);
        assert_eq!(p.shrink, 1.0);
        assert_eq!(p.basis, Basis::Auto);
        assert_eq!(p.align_self, AlignSelf::Auto);
        assert_eq!(p.fill_main, 0);
        assert_eq!(p.fill_cross, 0);
    }

    #[test]
    fn negative_grow_shrink_clamp_to_zero() {
        // The builder methods clamp negative values; mirror that
        // semantics here against Properties directly.
        let p = Properties {
            grow: (-1.0_f32).max(0.0),
            shrink: (-2.0_f32).max(0.0),
            ..Properties::default()
        };
        assert_eq!(p.grow, 0.0);
        assert_eq!(p.shrink, 0.0);
    }

    #[test]
    fn pixels_basis_is_set() {
        let p = Properties {
            basis: Basis::Pixels(120.0),
            ..Properties::default()
        };
        assert_eq!(p.basis, Basis::Pixels(120.0));
    }

    #[test]
    fn resolved_properties_explicit_basis_pixels_overrides_inner_fill_main() {
        // Regression: when a child's inner widget has Length::Fill in
        // the main axis but the user has set an explicit basis(N), the
        // engine must classify the item as fixed-main (fill_main=0),
        // not fluid.
        //
        // Otherwise, a sibling with grow=1 swallows all the available
        // space because the basis item falls into Pass 3's fluid
        // bucket with grow=0 and gets share_main=0 — rendering at
        // zero width. Reported via the flex-basis and kitchen-sink
        // example demos.
        use crate::core::{Length, Theme};
        use iced_widget::{Renderer, Space};

        let fluid: crate::core::Element<'_, (), Theme, Renderer> =
            Space::new().width(Length::Fill).height(Length::Fill).into();

        let child = FlexChild::new(fluid).basis(80.0);
        let props = child.resolved_properties(Axis::Horizontal);

        assert_eq!(
            props.fill_main, 0,
            "explicit Basis::Pixels must override the inner widget's \
             Length::Fill main hint"
        );
        assert_eq!(props.basis, Basis::Pixels(80.0));
        assert_eq!(props.grow, 0.0);
    }
}
