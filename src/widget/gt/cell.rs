//! A typed value occupying a single position in a [`Table`](super::Table).

/// A typed value occupying a single position in a [`Table`](super::Table).
///
/// `Number` cells are passed through any [`Formatter`](super::Formatter)
/// applied to their column at render time. `Text` cells render verbatim;
/// numeric formatters skip them. `Empty` is rendered by the formatter for
/// numeric columns (default `"—"`) and as the empty string for text
/// columns.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A numeric value, formatted at render time.
    Number(f64),
    /// A text value, rendered verbatim in text columns and passed through
    /// numeric formatters unchanged.
    Text(String),
    /// An empty value. Rendered by the formatter for numeric columns;
    /// renders as the empty string in text columns.
    Empty,
}

impl Cell {
    /// Creates a [`Cell::Number`] from any numeric value convertible to `f64`.
    pub fn number(n: impl Into<f64>) -> Self {
        Self::Number(n.into())
    }

    /// Creates a [`Cell::Text`] from any value convertible to `String`.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }
}

impl From<f64> for Cell {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i64> for Cell {
    fn from(value: i64) -> Self {
        Self::Number(value as f64)
    }
}

impl From<i32> for Cell {
    fn from(value: i32) -> Self {
        Self::Number(value as f64)
    }
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
