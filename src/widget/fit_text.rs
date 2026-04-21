//! Text that scales its font size to fit the bounds it is given.
//!
//! [`FitText`] is a drop-in-ish replacement for `iced::widget::text` where
//! you hand it a range `[min_size, max_size]` and it picks the largest font
//! size in that range whose shaped paragraph still fits inside the widget's
//! laid-out bounds. Think CSS' `clamp(min, ideal, max)`, but the "ideal" is
//! solved for instead of specified.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; pub use iced_widget::core::Length::Fill; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! use iced::Fill;
//! use sweeten::widget::fit_text;
//!
//! enum Message {}
//!
//! fn view<'a>() -> Element<'a, Message> {
//!     fit_text("BIG HEADLINE")
//!         .max_size(120)
//!         .min_size(16)
//!         .width(Fill)
//!         .height(96.0)
//!         .center()
//!         .into()
//! }
//! ```
//!
//! # How it works
//!
//! [`FitText`] binary-searches the size range using the renderer's
//! [`Paragraph`] implementation. Each probe builds a throwaway paragraph
//! at a candidate size and checks [`Paragraph::min_bounds`] against the
//! fit bounds — so it's entirely in the widget layer, no renderer or
//! `cosmic-text` changes required.
//!
//! Bounds follow the axis semantics of the widget's [`Length`] settings:
//! a `Fill` or `Fixed` axis is fit-bounded, a `Shrink` axis is treated as
//! unconstrained for fit purposes (since "shrink to content at max size"
//! is the intuitive behavior there). So the canonical usage is one of:
//!
//! - `width(Fill)` + `wrapping(None)` — single-line fit that scales to
//!   the available width
//! - `width(Fill).height(Fill)` + `wrapping(Word)` — multi-line fit that
//!   scales to both axes
//!
//! # Caveat
//!
//! With word wrapping, `fits(size)` is only approximately monotonic — a
//! slightly larger font can reflow to fewer or more lines, flipping the
//! predicate non-monotonically. Binary search still converges to a safe
//! size (one that fits), but may pick a value a hair below the true
//! optimum. In practice the error is imperceptible.

use crate::core::alignment;
use crate::core::font;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::renderer;
use crate::core::text;
use crate::core::text::paragraph::{self, Paragraph};
use crate::core::widget::text as core_text;
use crate::core::widget::tree::{self, Tree};
use crate::core::{
    Color, Element, Em, Length, Pixels, Rectangle, Size, Widget,
};

pub use core_text::{
    Alignment, Catalog, Ellipsis, LineHeight, Shaping, Style, StyleFn, Wrapping,
};

/// Text that scales its font size to fit its laid-out bounds.
///
/// See the [module docs](self) for details.
#[must_use]
pub struct FitText<'a, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fragment: text::Fragment<'a>,
    format: core_text::Format<Renderer::Font>,
    min_size: Option<Pixels>,
    max_size: Option<Pixels>,
    class: Theme::Class<'a>,
}

/// Effective floor used when [`FitText::min_size`] is not set.
const DEFAULT_MIN_SIZE: Pixels = Pixels(1.0);

/// Effective cap used when [`FitText::max_size`] is not set.
const DEFAULT_MAX_SIZE: Pixels = Pixels(1024.0);

impl<'a, Theme, Renderer> FitText<'a, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// Creates a new [`FitText`] from the given fragment.
    ///
    /// With no explicit [`min_size`](Self::min_size) or
    /// [`max_size`](Self::max_size), the fit search is effectively
    /// unbounded — the font scales within `[1.0, 1024.0]` pixels.
    pub fn new(fragment: impl text::IntoFragment<'a>) -> Self {
        let format = core_text::Format {
            width: Length::Fill,
            height: Length::Shrink,
            wrapping: Wrapping::None,
            ..core_text::Format::default()
        };

        FitText {
            fragment: fragment.into_fragment(),
            format,
            min_size: None,
            max_size: None,
            class: Theme::default(),
        }
    }

    /// Sets the maximum font size (the cap) used for the fit search.
    ///
    /// Defaults to `1024.0` pixels (effectively unbounded) if not set.
    pub fn max_size(mut self, size: impl Into<Pixels>) -> Self {
        self.max_size = Some(size.into());
        self
    }

    /// Sets the minimum font size (the floor) used for the fit search.
    ///
    /// If the content doesn't fit even at `min_size`, the text is rendered
    /// at `min_size` and may overflow — mirroring CSS' `clamp`.
    ///
    /// Defaults to `1.0` pixel if not set.
    pub fn min_size(mut self, size: impl Into<Pixels>) -> Self {
        self.min_size = Some(size.into());
        self
    }

    /// Sets the [`LineHeight`] of the [`FitText`].
    pub fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.format.line_height = line_height.into();
        self
    }

    /// Sets the [`Font`] of the [`FitText`].
    ///
    /// [`Font`]: text::Renderer::Font
    pub fn font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.format.font = Some(font.into());
        self
    }

    /// Sets the [`Font`] of the [`FitText`], if `Some`.
    ///
    /// [`Font`]: text::Renderer::Font
    pub fn font_maybe(
        mut self,
        font: Option<impl Into<Renderer::Font>>,
    ) -> Self {
        self.format.font = font.map(Into::into);
        self
    }

    /// Sets the font [`Weight`] of the [`FitText`] without changing the font
    /// family.
    ///
    /// [`Weight`]: font::Weight
    pub fn weight(mut self, weight: font::Weight) -> Self {
        self.format.weight = Some(weight);
        self
    }

    /// Sets the width of the [`FitText`] boundaries.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.format.width = width.into();
        self
    }

    /// Sets the height of the [`FitText`] boundaries.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.format.height = height.into();
        self
    }

    /// Centers the [`FitText`], both horizontally and vertically.
    pub fn center(self) -> Self {
        self.align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
    }

    /// Sets the horizontal alignment of the [`FitText`].
    pub fn align_x(mut self, alignment: impl Into<text::Alignment>) -> Self {
        self.format.align_x = alignment.into();
        self
    }

    /// Sets the vertical alignment of the [`FitText`].
    pub fn align_y(
        mut self,
        alignment: impl Into<alignment::Vertical>,
    ) -> Self {
        self.format.align_y = alignment.into();
        self
    }

    /// Sets the [`Shaping`] strategy of the [`FitText`].
    pub fn shaping(mut self, shaping: Shaping) -> Self {
        self.format.shaping = shaping;
        self
    }

    /// Sets the [`Wrapping`] strategy of the [`FitText`].
    pub fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.format.wrapping = wrapping;
        self
    }

    /// Sets the [`Ellipsis`] strategy of the [`FitText`].
    pub fn ellipsis(mut self, ellipsis: Ellipsis) -> Self {
        self.format.ellipsis = ellipsis;
        self
    }

    /// Sets the letter spacing of the [`FitText`].
    pub fn letter_spacing(mut self, letter_spacing: impl Into<Em>) -> Self {
        self.format.letter_spacing = letter_spacing.into();
        self
    }

    /// Adds a single font [`Feature`](font::Feature) to the [`FitText`].
    pub fn font_feature(mut self, feature: impl Into<font::Feature>) -> Self {
        self.format.font_features.push(feature.into());
        self
    }

    /// Sets the font features of the [`FitText`].
    pub fn font_features(mut self, features: Vec<font::Feature>) -> Self {
        self.format.font_features = features;
        self
    }

    /// Adds a single font [`Variation`](font::Variation) to the [`FitText`].
    pub fn font_variation(mut self, variation: font::Variation) -> Self {
        self.format.font_variations.push(variation);
        self
    }

    /// Sets the font variations of the [`FitText`].
    pub fn font_variations(mut self, variations: Vec<font::Variation>) -> Self {
        self.format.font_variations = variations;
        self
    }

    /// Sets the style of the [`FitText`].
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the [`Color`] of the [`FitText`].
    pub fn color(self, color: impl Into<Color>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.color_maybe(Some(color))
    }

    /// Sets the [`Color`] of the [`FitText`], if `Some`.
    pub fn color_maybe(self, color: Option<impl Into<Color>>) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        let color = color.map(Into::into);

        self.style(move |_theme| Style { color })
    }

    /// Sets the style class of the [`FitText`].
    #[cfg(feature = "advanced")]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

/// The internal state of a [`FitText`] widget.
///
/// Holds the shaped paragraph alongside a cache of the last fit decision so
/// we only re-run the search when an input actually changes.
pub struct State<P: Paragraph> {
    paragraph: paragraph::Plain<P>,
    cache: Option<FitCache<P::Font>>,
}

impl<P: Paragraph> Default for State<P> {
    fn default() -> Self {
        Self {
            paragraph: paragraph::Plain::default(),
            cache: None,
        }
    }
}

/// Everything a fit result is a function of — if all of these match the
/// last probe, we can reuse the previously chosen size.
#[derive(Clone)]
struct FitCache<Font> {
    content: String,
    fit_bounds: Size,
    font: Font,
    weight: Option<font::Weight>,
    line_height: LineHeight,
    letter_spacing: Em,
    shaping: Shaping,
    wrapping: Wrapping,
    ellipsis: Ellipsis,
    font_features: Vec<font::Feature>,
    font_variations: Vec<font::Variation>,
    hint_factor: Option<f32>,
    min_size: Pixels,
    max_size: Pixels,
    chosen: Pixels,
}

impl<Font: PartialEq> FitCache<Font> {
    fn matches(&self, probe: &FitCache<Font>) -> bool {
        self.content == probe.content
            && size_eq(self.fit_bounds, probe.fit_bounds)
            && self.font == probe.font
            && self.weight == probe.weight
            && self.line_height == probe.line_height
            && self.letter_spacing == probe.letter_spacing
            && self.shaping == probe.shaping
            && self.wrapping == probe.wrapping
            && self.ellipsis == probe.ellipsis
            && self.font_features == probe.font_features
            && self.font_variations == probe.font_variations
            && self.hint_factor == probe.hint_factor
            && self.min_size == probe.min_size
            && self.max_size == probe.max_size
    }
}

fn size_eq(a: Size, b: Size) -> bool {
    a.width.to_bits() == b.width.to_bits()
        && a.height.to_bits() == b.height.to_bits()
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for FitText<'_, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.format.width,
            height: self.format.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        layout::sized(limits, self.format.width, self.format.height, |limits| {
            let bounds = limits.max();
            let compression = limits.compression();

            // A Shrink axis has no meaningful "fit bound"; leave it
            // unconstrained so the text can grow to max_size there.
            let fit_bounds = Size::new(
                if compression.width {
                    f32::INFINITY
                } else {
                    bounds.width
                },
                if compression.height {
                    f32::INFINITY
                } else {
                    bounds.height
                },
            );

            let font =
                self.format.font.unwrap_or_else(|| renderer.default_font());
            let hint_factor = renderer.scale_factor();

            let min = self.min_size.unwrap_or(DEFAULT_MIN_SIZE);
            let max = self.max_size.unwrap_or(DEFAULT_MAX_SIZE);

            // Clamp so min <= max even if the user passed them backwards.
            let min_size = Pixels(min.0.min(max.0));
            let max_size = Pixels(min.0.max(max.0));

            let probe_key = FitCache {
                content: self.fragment.to_string(),
                fit_bounds,
                font,
                weight: self.format.weight,
                line_height: self.format.line_height,
                letter_spacing: self.format.letter_spacing,
                shaping: self.format.shaping,
                wrapping: self.format.wrapping,
                ellipsis: self.format.ellipsis,
                font_features: self.format.font_features.clone(),
                font_variations: self.format.font_variations.clone(),
                hint_factor,
                min_size,
                max_size,
                chosen: Pixels::ZERO, // filled in below
            };

            let chosen = match &state.cache {
                Some(cached) if cached.matches(&probe_key) => cached.chosen,
                _ => {
                    let chosen = fit::<Renderer::Paragraph>(
                        &self.fragment,
                        fit_bounds,
                        font,
                        min_size,
                        max_size,
                        &self.format,
                        hint_factor,
                    );
                    state.cache = Some(FitCache {
                        chosen,
                        ..probe_key
                    });
                    chosen
                }
            };

            // Commit the chosen size into the cached paragraph.
            let _ = state.paragraph.update(text::Text {
                content: &self.fragment,
                bounds,
                size: chosen,
                line_height: self.format.line_height,
                font,
                align_x: self.format.align_x,
                align_y: self.format.align_y,
                shaping: self.format.shaping,
                wrapping: self.format.wrapping,
                ellipsis: self.format.ellipsis,
                letter_spacing: self.format.letter_spacing,
                font_features: self.format.font_features.clone(),
                font_variations: self.format.font_variations.clone(),
                weight: self.format.weight,
                hint_factor,
            });

            state.paragraph.min_bounds()
        })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor_position: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let style = theme.style(&self.class);

        core_text::draw(
            renderer,
            defaults,
            layout.bounds(),
            state.paragraph.raw(),
            style,
            viewport,
        );
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn crate::core::widget::Operation,
    ) {
        operation.text(None, layout.bounds(), &self.fragment);
    }
}

/// Binary-searches `[min_size, max_size]` for the largest font size whose
/// shaped paragraph fits inside `fit_bounds`.
fn fit<P: Paragraph>(
    content: &str,
    fit_bounds: Size,
    font: P::Font,
    min_size: Pixels,
    max_size: Pixels,
    format: &core_text::Format<P::Font>,
    hint_factor: Option<f32>,
) -> Pixels {
    // Slight tolerance so 0.4px overrun doesn't reject an otherwise-fine
    // size — cosmic-text's measurements vary sub-pixel between probes.
    let tol = 0.5;

    let probe = |size: Pixels| -> Size {
        let p = P::with_text(text::Text {
            content,
            bounds: fit_bounds,
            size,
            line_height: format.line_height,
            font,
            align_x: format.align_x,
            align_y: format.align_y,
            shaping: format.shaping,
            wrapping: format.wrapping,
            ellipsis: format.ellipsis,
            letter_spacing: format.letter_spacing,
            font_features: format.font_features.clone(),
            font_variations: format.font_variations.clone(),
            weight: format.weight,
            hint_factor,
        });
        p.min_bounds()
    };

    let fits = |mb: Size| -> bool {
        mb.width <= fit_bounds.width + tol
            && mb.height <= fit_bounds.height + tol
    };

    // Fast path: if the ceiling already fits, we're done.
    if fits(probe(max_size)) {
        return max_size;
    }
    // Floor path: if even the floor doesn't fit, bottom out at it.
    if !fits(probe(min_size)) {
        return min_size;
    }

    // Binary search to ~0.5px precision.
    let mut lo = min_size.0;
    let mut hi = max_size.0;
    while hi - lo > 0.5 {
        let mid = (lo + hi) * 0.5;
        if fits(probe(Pixels(mid))) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Pixels(lo)
}

impl<'a, Message, Theme, Renderer> From<FitText<'a, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(
        text: FitText<'a, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(text)
    }
}
