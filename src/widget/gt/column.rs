//! Column descriptors for a [`Table`](super::Table).

use crate::core::Length;
use crate::core::alignment;

/// Descriptor for a column in a [`Table`](super::Table).
///
/// The `kind` drives default alignment (`Numeric` → right, `Text` → left)
/// and decides whether numeric formatters apply to the column's cells.
#[derive(Debug, Clone)]
pub struct Column {
    /// Stable selector key. Used by
    /// [`Selector::columns`](super::Selector::columns).
    pub id: String,
    /// Display text for the column header.
    pub label: String,
    /// Whether the column holds numeric or text data.
    pub kind: ColumnKind,
    /// Width directive for the column.
    pub width: Length,
    /// Optional horizontal alignment override. `None` falls back to the
    /// kind default (`Numeric` → right, `Text` → left).
    pub align: Option<alignment::Horizontal>,
}

/// Whether a [`Column`] holds numeric or text data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    /// Numeric column. Cells run through any matching
    /// [`Formatter`](super::Formatter) at render time.
    Numeric,
    /// Text column. Cells render verbatim; numeric formatters do not apply.
    Text,
}

impl Column {
    /// Creates a numeric column with the given id and label.
    pub fn numeric(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ColumnKind::Numeric,
            width: Length::Shrink,
            align: None,
        }
    }

    /// Creates a text column with the given id and label.
    pub fn text(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ColumnKind::Text,
            width: Length::Shrink,
            align: None,
        }
    }

    /// Sets the column's [`Length`] directive.
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets an alignment override for the column. `None` restores the
    /// kind default.
    #[must_use]
    pub fn align(mut self, align: alignment::Horizontal) -> Self {
        self.align = Some(align);
        self
    }
}

impl ColumnKind {
    /// Returns the default horizontal alignment for the column kind.
    pub fn default_align(self) -> alignment::Horizontal {
        match self {
            Self::Numeric => alignment::Horizontal::Right,
            Self::Text => alignment::Horizontal::Left,
        }
    }
}
