//! Caller-computed summary rows for a [`Table`](super::Table).

use super::Cell;

/// A row of pre-computed summary values rendered below a group's data
/// rows or below the entire table.
///
/// `gt::Table` does not auto-sum — the caller materializes summary
/// values from its own data model and passes them in. When `group_id`
/// is `Some`, the row renders directly below the matching group; when
/// `None`, the row renders in the grand-summary block below all data.
#[derive(Debug, Clone)]
pub struct SummaryRow {
    /// Display label rendered in the stub column (when set) or the
    /// leftmost column otherwise.
    pub label: String,
    /// Group id this summary belongs to, or `None` for a grand summary.
    pub group_id: Option<String>,
    /// Positional cells, one per column in the [`Table`](super::Table).
    pub cells: Vec<Cell>,
}

impl SummaryRow {
    /// Creates a grand-summary row with the given label and cells.
    pub fn grand(label: impl Into<String>, cells: Vec<Cell>) -> Self {
        Self {
            label: label.into(),
            group_id: None,
            cells,
        }
    }

    /// Creates a group-summary row attached to the given group id.
    pub fn group(
        group_id: impl Into<String>,
        label: impl Into<String>,
        cells: Vec<Cell>,
    ) -> Self {
        Self {
            label: label.into(),
            group_id: Some(group_id.into()),
            cells,
        }
    }
}
