//! Style records for [`Table`](super::Table) cells, plus the
//! table-level [`Catalog`] and default [`Style`].

use crate::core::{Background, Color, Padding, alignment, font};

/// All-optional per-cell style record. [`Table::tab_style`] field-merges
/// these — only `Some` fields override prior styles at overlapping cells;
/// unset fields preserve prior. [`TextStyle::features`] concatenate
/// rather than replacing.
///
/// [`Table::tab_style`]: super::Table::tab_style
#[derive(Debug, Default, Clone)]
pub struct CellStyle {
    /// Text-level overrides applied to the cell's rendered text.
    pub text: Option<TextStyle>,
    /// Background fill drawn behind the cell.
    pub fill: Option<Background>,
    /// Per-cell border overrides drawn at the cell's edges, centered
    /// on the edge in CSS-`border-collapse: collapse` style.
    pub borders: Option<BorderStyle>,
    // TODO(v2): per-cell padding overrides. Reserved field — currently
    // a no-op. Wiring it requires growing the layout pass to track
    // per-cell content area independently of the cell stride (so
    // column widths still align across rows when one row's cells use
    // different padding). Today every cell uses the table-level
    // `padding_x` / `padding_y`.
    /// Reserved for future per-cell padding overrides. **Currently a
    /// no-op** — every cell uses the table-level
    /// [`Table::padding_x`](super::Table::padding_x) /
    /// [`Table::padding_y`](super::Table::padding_y).
    pub padding: Option<Padding>,
}

/// All-optional text style overrides applied to a cell's rendered text.
#[derive(Debug, Default, Clone)]
pub struct TextStyle {
    /// Font size in pixels.
    pub size: Option<f32>,
    /// Font weight (e.g. `font::Weight::Bold`).
    pub weight: Option<font::Weight>,
    /// Font style (italic, oblique).
    pub style: Option<font::Style>,
    /// Font family override.
    pub family: Option<font::Family>,
    /// Text color.
    pub color: Option<Color>,
    /// Horizontal alignment override. Falls back to the column's
    /// configured alignment when unset.
    pub align: Option<alignment::Horizontal>,
    /// Optional case transform applied to the rendered string.
    pub transform: Option<TextTransform>,
    /// Letter spacing in pixels (`em` units accepted via `Into<Em>`).
    pub letter_spacing: Option<f32>,
    /// OpenType font features (e.g. `tnum` for tabular figures).
    /// Concatenated across overlapping `tab_style` calls rather than
    /// replaced, so callers can layer features.
    pub features: Vec<font::Feature>,
}

/// All-optional border overrides drawn at a cell's edges.
///
/// `sides` selects which edges receive the border; `color` and `width`
/// apply to whatever sides are selected.
#[derive(Debug, Default, Clone, Copy)]
pub struct BorderStyle {
    /// Which edges to draw.
    pub sides: Sides,
    /// Border color.
    pub color: Option<Color>,
    /// Border thickness in pixels.
    pub width: Option<f32>,
}

/// Per-edge selection for [`BorderStyle::sides`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sides {
    /// Whether to draw the top edge.
    pub top: bool,
    /// Whether to draw the right edge.
    pub right: bool,
    /// Whether to draw the bottom edge.
    pub bottom: bool,
    /// Whether to draw the left edge.
    pub left: bool,
}

/// Case transform applied to a cell's rendered text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextTransform {
    /// No transform.
    None,
    /// Uppercase the text.
    Uppercase,
    /// Lowercase the text.
    Lowercase,
}

impl Sides {
    /// All four edges enabled.
    pub const fn all() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }

    /// Only the top edge enabled.
    pub const fn top() -> Self {
        Self {
            top: true,
            right: false,
            bottom: false,
            left: false,
        }
    }

    /// Only the bottom edge enabled.
    pub const fn bottom() -> Self {
        Self {
            top: false,
            right: false,
            bottom: true,
            left: false,
        }
    }

    /// Only the left edge enabled.
    pub const fn left() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: true,
        }
    }

    /// Only the right edge enabled.
    pub const fn right() -> Self {
        Self {
            top: false,
            right: true,
            bottom: false,
            left: false,
        }
    }
}

impl CellStyle {
    /// Field-merges `other` into `self`. Only `Some` fields in `other`
    /// override their counterparts in `self`. Vec fields concatenate.
    pub(super) fn merge(&mut self, other: &CellStyle) {
        if let Some(other_text) = &other.text {
            match self.text.as_mut() {
                Some(text) => text.merge(other_text),
                None => self.text = Some(other_text.clone()),
            }
        }
        if let Some(fill) = other.fill {
            self.fill = Some(fill);
        }
        if let Some(borders) = other.borders {
            match self.borders.as_mut() {
                Some(self_borders) => self_borders.merge(&borders),
                None => self.borders = Some(borders),
            }
        }
        if let Some(padding) = other.padding {
            self.padding = Some(padding);
        }
    }
}

impl TextStyle {
    fn merge(&mut self, other: &TextStyle) {
        if let Some(size) = other.size {
            self.size = Some(size);
        }
        if let Some(weight) = other.weight {
            self.weight = Some(weight);
        }
        if let Some(style) = other.style {
            self.style = Some(style);
        }
        if let Some(family) = other.family {
            self.family = Some(family);
        }
        if let Some(color) = other.color {
            self.color = Some(color);
        }
        if let Some(align) = other.align {
            self.align = Some(align);
        }
        if let Some(transform) = other.transform {
            self.transform = Some(transform);
        }
        if let Some(letter_spacing) = other.letter_spacing {
            self.letter_spacing = Some(letter_spacing);
        }
        self.features.extend(other.features.iter().copied());
    }
}

impl BorderStyle {
    fn merge(&mut self, other: &BorderStyle) {
        if other.sides != Sides::default() {
            self.sides = other.sides;
        }
        if let Some(color) = other.color {
            self.color = Some(color);
        }
        if let Some(width) = other.width {
            self.width = Some(width);
        }
    }
}

/// Table-level appearance for a [`Table`](super::Table).
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Color of the horizontal line separator drawn between rows.
    pub separator_x: Background,
    /// Color of the vertical line separator drawn between columns.
    pub separator_y: Background,
    /// Color of the outline drawn around the entire table when
    /// [`Table::border`](super::Table::border) is non-zero.
    pub border: Background,
    /// Background fill drawn behind the sticky header block, so that
    /// scrolling data rows don't show through.
    pub sticky_background: Background,
    /// Default text color when no [`TextStyle::color`] override applies.
    pub text: Color,
    /// Color used for the title block text when no override applies.
    pub title: Color,
    /// Color used for subtitle / units-caption / source-notes text.
    pub muted: Color,
}

/// Theme catalog for a [`Table`](super::Table).
pub trait Catalog {
    /// The class type produced by the catalog.
    type Class<'a>;

    /// The default class produced by the catalog.
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] for the given class.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`Table`](super::Table).
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl<Theme> From<Style> for StyleFn<'_, Theme> {
    fn from(style: Style) -> Self {
        Box::new(move |_theme| style)
    }
}

impl Catalog for crate::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// The default [`Style`] for a [`Table`](super::Table).
///
/// Minimal chrome: hairline separators, no fills, no borders. Consumers
/// add visual emphasis through [`Table::tab_style`] selector rules.
///
/// [`Table::tab_style`]: super::Table::tab_style
pub fn default(theme: &crate::Theme) -> Style {
    let palette = theme.palette();
    Style {
        separator_x: palette.background.weak.color.into(),
        separator_y: palette.background.weak.color.into(),
        border: palette.background.weak.color.into(),
        sticky_background: palette.background.base.color.into(),
        text: palette.background.base.text,
        title: palette.background.base.text,
        muted: palette.background.weakest.text,
    }
}
