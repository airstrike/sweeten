//! Selectors over the cell-coordinate space of a
//! [`Table`](super::Table).
//!
//! A [`Selector`] picks a subset of cells (or full-width rows) to be
//! styled or formatted. Build one starting from the [`cells`] namespace,
//! then refine with [`Selector::columns`] / [`Selector::rows`] /
//! [`Selector::groups`], and combine with [`Selector::intersect`] /
//! [`Selector::union`].
//!
//! Refinement methods are last-write-wins per axis: calling
//! `.columns(["a"]).columns(["b"])` keeps only `["b"]`. Cross-selector
//! combinations are explicit via `intersect` / `union`.

use std::collections::HashSet;
use std::sync::Arc;

/// A selector over the cell-coordinate space of a
/// [`Table`](super::Table).
#[derive(Clone)]
pub struct Selector {
    repr: Repr,
}

#[derive(Clone)]
enum Repr {
    Atom(Atom),
    Intersect(Box<Selector>, Box<Selector>),
    Union(Box<Selector>, Box<Selector>),
}

#[derive(Clone)]
struct Atom {
    layer: Layer,
    columns: Option<HashSet<String>>,
    groups: Option<HashSet<String>>,
    predicate: Option<Predicate>,
}

type Predicate = Arc<dyn Fn(usize) -> bool + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Layer {
    Body,
    ColumnLabels,
    Stub,
    Summary,
    GrandSummary,
    RowGroupLabels,
    Title,
    Subtitle,
    UnitsCaption,
    SourceNotes,
}

/// Coordinate passed to [`Selector::matches`]. The table builds one of
/// these for each cell (or spanned row) it draws and asks every
/// accumulated selector whether it applies.
#[derive(Debug, Clone, Copy)]
pub(super) struct Coord<'a> {
    pub layer: Layer,
    pub row: usize,
    pub column_id: Option<&'a str>,
    pub group_id: Option<&'a str>,
}

impl Selector {
    fn atom(layer: Layer) -> Self {
        Self {
            repr: Repr::Atom(Atom {
                layer,
                columns: None,
                groups: None,
                predicate: None,
            }),
        }
    }

    /// Restricts the selector to the given column ids. Last-write-wins
    /// — calling `.columns()` again replaces the previous restriction.
    /// Use [`intersect`](Self::intersect) for set intersection across
    /// selectors.
    #[must_use]
    pub fn columns(
        self,
        ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let ids: HashSet<String> = ids.into_iter().map(Into::into).collect();
        self.with_atom(|atom| atom.columns = Some(ids))
    }

    /// Restricts the selector by a row predicate. The predicate is
    /// called per row index within the selector's layer (e.g. body
    /// rows for [`cells::body`], summary rows for [`cells::summary`]).
    /// Last-write-wins.
    #[must_use]
    pub fn rows(
        self,
        predicate: impl Fn(usize) -> bool + Send + Sync + 'static,
    ) -> Self {
        let predicate: Predicate = Arc::new(predicate);
        self.with_atom(|atom| atom.predicate = Some(predicate))
    }

    /// Restricts the selector to cells belonging to the given row-group
    /// ids. Last-write-wins.
    #[must_use]
    pub fn groups(
        self,
        ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let ids: HashSet<String> = ids.into_iter().map(Into::into).collect();
        self.with_atom(|atom| atom.groups = Some(ids))
    }

    /// Intersects this selector with `other`. A cell matches the
    /// resulting selector when both children match.
    #[must_use]
    pub fn intersect(self, other: Selector) -> Self {
        Self {
            repr: Repr::Intersect(Box::new(self), Box::new(other)),
        }
    }

    /// Unions this selector with `other`. A cell matches the resulting
    /// selector when either child matches.
    #[must_use]
    pub fn union(self, other: Selector) -> Self {
        Self {
            repr: Repr::Union(Box::new(self), Box::new(other)),
        }
    }

    fn with_atom(mut self, f: impl FnOnce(&mut Atom)) -> Self {
        match &mut self.repr {
            Repr::Atom(atom) => f(atom),
            Repr::Intersect(_, _) | Repr::Union(_, _) => {
                // FIXME(v2): refinement on a composite selector
                // currently degrades to "intersect with a body atom
                // carrying the refinement". Composite selectors don't
                // carry a single layer, so we can't push the column /
                // row / group constraint into every leaf without
                // walking the tree. Callers should refine the leaves
                // before composing — e.g.
                // `cells::body().columns(["a"]).union(cells::summary().columns(["a"]))`,
                // not `cells::body().union(cells::summary()).columns(["a"])`.
                // Lift to first-class refinement on composites if a
                // real use case appears.
                let mut atom = Atom {
                    layer: Layer::Body,
                    columns: None,
                    groups: None,
                    predicate: None,
                };
                f(&mut atom);
                let refined = Self {
                    repr: Repr::Atom(atom),
                };
                self = self.intersect(refined);
            }
        }
        self
    }

    pub(super) fn matches(
        &self,
        coord: &Coord<'_>,
        stub_column: Option<&str>,
    ) -> bool {
        match &self.repr {
            Repr::Atom(atom) => match_atom(atom, coord, stub_column),
            Repr::Intersect(a, b) => {
                a.matches(coord, stub_column) && b.matches(coord, stub_column)
            }
            Repr::Union(a, b) => {
                a.matches(coord, stub_column) || b.matches(coord, stub_column)
            }
        }
    }
}

fn match_atom(
    atom: &Atom,
    coord: &Coord<'_>,
    stub_column: Option<&str>,
) -> bool {
    let layer_ok = match atom.layer {
        Layer::Stub => {
            coord.layer == Layer::Body && coord.column_id == stub_column
        }
        Layer::Body
        | Layer::ColumnLabels
        | Layer::Summary
        | Layer::GrandSummary
        | Layer::RowGroupLabels
        | Layer::Title
        | Layer::Subtitle
        | Layer::UnitsCaption
        | Layer::SourceNotes => coord.layer == atom.layer,
    };
    if !layer_ok {
        return false;
    }
    if let Some(cols) = &atom.columns {
        match coord.column_id {
            Some(id) if cols.contains(id) => {}
            _ => return false,
        }
    }
    if let Some(groups) = &atom.groups {
        match coord.group_id {
            Some(id) if groups.contains(id) => {}
            _ => return false,
        }
    }
    if let Some(pred) = &atom.predicate
        && !pred(coord.row)
    {
        return false;
    }
    true
}

/// Pre-built selectors targeting the named layers of a
/// [`Table`](super::Table).
pub mod cells {
    use super::{Layer, Selector};

    /// Every body cell (excludes header, summary, source notes, etc.).
    /// The stub column is part of the body.
    pub fn body() -> Selector {
        Selector::atom(Layer::Body)
    }

    /// The header label row.
    pub fn column_labels() -> Selector {
        Selector::atom(Layer::ColumnLabels)
    }

    /// Body cells in the stub column. Resolves at render time to the
    /// column id passed to
    /// [`Table::stub_column`](super::super::Table::stub_column); a
    /// table with no stub column matches no cells.
    pub fn stub() -> Selector {
        Selector::atom(Layer::Stub)
    }

    /// Cells in any group-summary row.
    pub fn summary() -> Selector {
        Selector::atom(Layer::Summary)
    }

    /// Cells in any grand-summary row.
    pub fn grand_summary() -> Selector {
        Selector::atom(Layer::GrandSummary)
    }

    /// The optional group-header rows above each row group.
    pub fn row_group_labels() -> Selector {
        Selector::atom(Layer::RowGroupLabels)
    }

    /// The title block above the table.
    pub fn title() -> Selector {
        Selector::atom(Layer::Title)
    }

    /// The subtitle row.
    pub fn subtitle() -> Selector {
        Selector::atom(Layer::Subtitle)
    }

    /// The units-caption row.
    pub fn units_caption() -> Selector {
        Selector::atom(Layer::UnitsCaption)
    }

    /// The source-notes block below the table.
    pub fn source_notes() -> Selector {
        Selector::atom(Layer::SourceNotes)
    }
}
