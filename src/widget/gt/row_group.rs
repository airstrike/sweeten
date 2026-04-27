//! Named groups of body rows in a [`Table`](super::Table).

/// A named block of body rows with an optional group-header row above.
///
/// When a [`Table`](super::Table) has any [`RowGroup`]s, rows are
/// rendered group-by-group in the order the groups are passed to
/// [`Table::row_groups`](super::Table::row_groups). Each group renders
/// its `row_indices` in the order they appear, and rows not assigned to
/// any group are dropped.
#[derive(Debug, Clone)]
pub struct RowGroup {
    /// Stable selector key. Used by
    /// [`Selector::groups`](super::Selector::groups).
    pub id: String,
    /// Optional label for a group-header row above the group's rows.
    /// `None` skips the header row entirely.
    pub label: Option<String>,
    /// Indices into the `rows` vector passed to
    /// [`Table::new`](super::Table::new), in the order they should
    /// appear within this group.
    pub row_indices: Vec<usize>,
}

impl RowGroup {
    /// Creates a new [`RowGroup`] with the given id and row indices.
    /// The group has no header row by default; call
    /// [`label`](Self::label) to add one.
    pub fn new(id: impl Into<String>, row_indices: Vec<usize>) -> Self {
        Self {
            id: id.into(),
            label: None,
            row_indices,
        }
    }

    /// Sets the group-header label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}
