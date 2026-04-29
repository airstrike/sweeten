//! A grammar-of-tables widget for richly-styled tables.
//!
//! This is a sweeten-only widget (no upstream counterpart in
//! `iced_widget`). The flat [`Table`](super::table::Table) stays as the
//! ergonomic choice for plain data grids; reach for [`gt::Table`] when
//! you need the full grammar — title / subtitle / units caption /
//! column labels / stub / body / row groups / summary / grand summary /
//! source notes — with selector-based styling and pluggable number
//! formatters.
//!
//! # Layer composition
//!
//! Vertical stacking order, top to bottom:
//!
//! 1. Title
//! 2. Subtitle
//! 3. Units caption
//! 4. Column labels
//! 5. Body, with per-row-group ordering when [`Table::row_groups`] is set:
//!    - Optional group header row
//!    - Group body rows (in the order specified by the group)
//!    - Group summary rows
//! 6. Grand summary rows
//! 7. Source notes
//!
//! Column widths are computed once across all per-column rows so
//! columns stay aligned across header / body / summary blocks.
//!
//! # Selector grammar
//!
//! Layout and styling are decoupled. Build a [`Selector`] starting from
//! the [`cells`] namespace ([`cells::body`], [`cells::column_labels`],
//! [`cells::stub`], etc.) and refine with [`Selector::columns`],
//! [`Selector::rows`], or [`Selector::groups`]. Apply via
//! [`Table::tab_style`] for cell styles or [`Table::fmt`] for number
//! formatters. Multiple `tab_style` calls accumulate; later calls
//! field-merge over earlier ones at overlapping cells.
//!
//! # Click handling
//!
//! [`Table::on_press`] mirrors [`Table::tab_style`] / [`Table::fmt`]:
//! pass a [`Selector`] and a closure that builds a `Message` from a
//! [`Click`]. Multiple `on_press` calls accumulate; on a press,
//! handlers are walked in registration order and the FIRST match
//! fires (so register specific handlers before broad fallbacks).
//! Cells matching no registered selector are non-clickable and show
//! no pointer cursor.
//!
//! ```ignore
//! gt::Table::new(columns, rows)
//!     .on_press(cells::body().columns(["country"]),
//!               |c| Message::DrillCountry(c.coord.row))
//!     .on_press(cells::body(),
//!               |c| Message::SelectCell(c.coord.row,
//!                                       c.coord.column.unwrap_or("")
//!                                                     .to_owned()))
//! ```
//!
//! # v2 roadmap
//!
//! Out of scope for v1 but tracked as TODOs:
//!
//! - **Spanner columns** — multi-level column headers that group
//!   adjacent columns under a shared label (`gt::tab_spanner` in R's
//!   `gt`). Needs a column-tree representation in [`Table::new`] and
//!   an extra header row in the layout pass.
//! - **Sorting** — clickable column-label cells that emit sort
//!   messages and a sorted view of `rows`. Needs `Message`-emitting
//!   cells (which v1 deliberately avoids).
//! - **Per-cell padding overrides** — [`CellStyle::padding`] is a
//!   reserved no-op field today. Wiring it requires per-cell content
//!   rectangles inside a uniform stride so column widths still align.
//!   The common case of "first/last column inset to align with an
//!   outer card title" is already covered by
//!   [`Table::outer_padding_x`].
//! - **Composite-selector refinement** — calling `.columns()` /
//!   `.rows()` / `.groups()` on a selector built via `intersect` /
//!   `union` currently degrades to "intersect with a body atom
//!   carrying the refinement" (see [`selector`] internals). Refining
//!   the leaves before composing is the supported pattern; lift this
//!   to first-class refinement on composites if a real use case
//!   appears.
//! - **Unit tests** — the [`fmt`] module is covered, but the selector
//!   matcher and the layout pass are visually-verified-only via
//!   `examples/gt.rs`. Targeted tests for selector composition and
//!   layer stacking would harden the widget.

mod cell;
mod column;
mod fmt;
mod row_group;
mod selector;
mod style;
mod summary_row;

pub use cell::Cell;
pub use column::{Column, ColumnKind};
pub use fmt::{
    EMPTY_GLYPH, Formatter, arbitrary, currency, decimal, number,
    parens_for_negatives, percent, scaled, scientific,
};
pub use row_group::RowGroup;
pub use selector::{CellCoord, CellLayer, Selector, cells};
pub use style::{
    BorderStyle, Catalog, CellStyle, Sides, Style, StyleFn, TextStyle,
    TextTransform, default,
};
pub use summary_row::SummaryRow;

use std::sync::Arc;

use crate::core;
use crate::core::alignment;
use crate::core::keyboard;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget;
use crate::core::{
    Background, Color, Element, Em, Layout, Length, Pixels, Point, Rectangle,
    Size, Vector, Widget,
};

use iced_widget::text;

/// A click on a [`Table`] cell, delivered to handlers registered via
/// `on_press`. Carries the cell's [`CellCoord`] plus the modifier and
/// mouse-button state at press time.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct Click<'a> {
    /// The clicked cell's coordinate.
    pub coord: CellCoord<'a>,
    /// Modifier keys held at press time.
    pub modifiers: keyboard::Modifiers,
    /// Which mouse button was pressed.
    pub button: mouse::Button,
}

/// Boxed click handler stored in the [`Table`]'s `on_press` accumulator.
type Handler<'a, Message> = Box<dyn for<'c> Fn(Click<'c>) -> Message + 'a>;

/// Owned coordinate stored alongside [`CellMeta`] so the widget can
/// rebuild a [`CellCoord`] (which borrows column / group ids) at hit-test
/// time without re-walking the source data.
#[derive(Debug, Clone)]
struct OwnedCellCoord {
    layer: CellLayer,
    row: usize,
    column: Option<String>,
    group: Option<String>,
}

impl OwnedCellCoord {
    fn as_borrowed(&self) -> CellCoord<'_> {
        CellCoord {
            layer: self.layer,
            row: self.row,
            column: self.column.as_deref(),
            group: self.group.as_deref(),
        }
    }
}

/// Builder for a grammar-of-tables widget. See the [module
/// docs](self) for the full grammar.
pub struct Table<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: Catalog,
    Renderer: core::text::Renderer,
{
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    title: Option<String>,
    subtitle: Option<String>,
    units_caption: Option<String>,
    stub_column: Option<String>,
    row_groups: Vec<RowGroup>,
    group_summary_rows: Vec<SummaryRow>,
    grand_summary_rows: Vec<SummaryRow>,
    source_notes: Vec<String>,
    tab_styles: Vec<(Selector, CellStyle)>,
    formatters: Vec<(Selector, Formatter)>,
    on_press: Vec<(Selector, Handler<'a, Message>)>,
    width: Length,
    padding_x: f32,
    padding_y: f32,
    outer_padding_x: Option<f32>,
    separator_x: f32,
    separator_y: f32,
    sticky_header: bool,
    class: <Theme as Catalog>::Class<'a>,
    _phantom: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Theme, Renderer> Table<'a, Message, Theme, Renderer>
where
    Theme: Catalog + text::Catalog + 'a,
    <Theme as text::Catalog>::Class<'a>: From<text::StyleFn<'a, Theme>>,
    Renderer: core::text::Renderer<Font = core::Font> + 'a,
{
    /// Builds a [`Table`] from typed columns and rows.
    ///
    /// `rows[i]` must have exactly `columns.len()` cells. Mismatched
    /// rows are accepted but missing cells render as
    /// [`Cell::Empty`] and extras are ignored at render time.
    pub fn new(columns: Vec<Column>, rows: Vec<Vec<Cell>>) -> Self {
        Self {
            columns,
            rows,
            title: None,
            subtitle: None,
            units_caption: None,
            stub_column: None,
            row_groups: Vec::new(),
            group_summary_rows: Vec::new(),
            grand_summary_rows: Vec::new(),
            source_notes: Vec::new(),
            tab_styles: Vec::new(),
            formatters: Vec::new(),
            on_press: Vec::new(),
            width: Length::Shrink,
            padding_x: 10.0,
            padding_y: 5.0,
            outer_padding_x: None,
            separator_x: 0.0,
            separator_y: 1.0,
            sticky_header: false,
            class: <Theme as Catalog>::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sets the table title.
    #[must_use]
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Sets the table subtitle.
    #[must_use]
    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    /// Sets the table units-caption row.
    #[must_use]
    pub fn units_caption(mut self, text: impl Into<String>) -> Self {
        self.units_caption = Some(text.into());
        self
    }

    /// Designates the column with the given id as the stub column.
    /// Body cells in that column are matched by [`cells::stub`].
    #[must_use]
    pub fn stub_column(mut self, column_id: impl Into<String>) -> Self {
        self.stub_column = Some(column_id.into());
        self
    }

    /// Sets the row groups. See [`RowGroup`] for ordering rules.
    #[must_use]
    pub fn row_groups(mut self, groups: Vec<RowGroup>) -> Self {
        self.row_groups = groups;
        self
    }

    /// Sets the group-summary rows. Each row's
    /// [`SummaryRow::group_id`] must match a [`RowGroup::id`]; rows
    /// whose `group_id` is `None` or doesn't match are dropped.
    #[must_use]
    pub fn summary_rows(mut self, rows: Vec<SummaryRow>) -> Self {
        self.group_summary_rows = rows;
        self
    }

    /// Sets the grand-summary rows rendered below all data.
    #[must_use]
    pub fn grand_summary_rows(mut self, rows: Vec<SummaryRow>) -> Self {
        self.grand_summary_rows = rows;
        self
    }

    /// Sets the source-notes block rendered below the table.
    #[must_use]
    pub fn source_notes(mut self, notes: Vec<String>) -> Self {
        self.source_notes = notes;
        self
    }

    /// Applies `style` to every cell matching `target`. Multiple calls
    /// accumulate; later calls field-merge over earlier ones at
    /// overlapping cells.
    #[must_use]
    pub fn tab_style(mut self, target: Selector, style: CellStyle) -> Self {
        self.tab_styles.push((target, style));
        self
    }

    /// Applies `formatter` to numeric cells matching `target`. Later
    /// calls override earlier ones at overlapping cells.
    #[must_use]
    pub fn fmt(mut self, target: Selector, formatter: Formatter) -> Self {
        self.formatters.push((target, formatter));
        self
    }

    /// Registers a click handler for cells matching `target`. Multiple
    /// `on_press` calls accumulate; on a press, handlers are walked in
    /// registration order and the FIRST matching handler fires (so
    /// register specific handlers before broad fallbacks). Cells matching
    /// no registered selector are non-clickable and show no pointer
    /// cursor.
    #[must_use]
    pub fn on_press<F>(mut self, target: Selector, handler: F) -> Self
    where
        F: 'a + Fn(Click<'_>) -> Message,
    {
        self.on_press.push((target, Box::new(handler)));
        self
    }

    /// Sets the width of the [`Table`].
    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the horizontal cell padding.
    #[must_use]
    pub fn padding_x(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding_x = padding.into().0;
        self
    }

    /// Sets the vertical cell padding.
    #[must_use]
    pub fn padding_y(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding_y = padding.into().0;
        self
    }

    /// Sets the outer horizontal padding — the extra inset applied to
    /// the leftmost cell on its left edge and to the rightmost cell on
    /// its right edge. Inter-cell gaps remain at
    /// [`padding_x`](Self::padding_x).
    ///
    /// Useful for embedding a table inside a card whose title is
    /// itself inset from the card edge: set `outer_padding_x` to
    /// match the card's content padding so the table's first-column
    /// text and last-column text line up with the card title, while
    /// borders / fills still extend edge-to-edge across the table.
    /// Spanned-row content (title, subtitle, units caption, group
    /// labels, source notes) is also inset by `outer_padding_x` so it
    /// shares the same start/end as the first/last data column.
    ///
    /// Defaults to `padding_x` when unset (no outer inset).
    #[must_use]
    pub fn outer_padding_x(mut self, padding: impl Into<Pixels>) -> Self {
        self.outer_padding_x = Some(padding.into().0);
        self
    }

    /// Sets the thickness of the vertical separator drawn between
    /// columns. Defaults to `0.0`.
    #[must_use]
    pub fn separator_x(mut self, separator: impl Into<Pixels>) -> Self {
        self.separator_x = separator.into().0;
        self
    }

    /// Sets the thickness of the horizontal separator drawn between
    /// rows. Defaults to `1.0`.
    #[must_use]
    pub fn separator_y(mut self, separator: impl Into<Pixels>) -> Self {
        self.separator_y = separator.into().0;
        self
    }

    /// Pins the title block and column labels to the top of the visible
    /// area when the table is scrolled inside a parent scrollable.
    #[must_use]
    pub fn sticky_header(mut self, sticky: bool) -> Self {
        self.sticky_header = sticky;
        self
    }

    /// Sets the table-level style class.
    #[must_use]
    pub fn class(
        mut self,
        class: impl Into<<Theme as Catalog>::Class<'a>>,
    ) -> Self {
        self.class = class.into();
        self
    }

    /// Sets the table-level style via a closure.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        <Theme as Catalog>::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    fn build(self) -> Materialized<'a, Message, Theme, Renderer> {
        let Self {
            columns,
            rows,
            title,
            subtitle,
            units_caption,
            stub_column,
            row_groups,
            group_summary_rows,
            grand_summary_rows,
            source_notes,
            tab_styles,
            formatters,
            on_press,
            width,
            padding_x,
            padding_y,
            outer_padding_x,
            separator_x,
            separator_y,
            sticky_header,
            class,
            _phantom,
        } = self;

        let stub_id = stub_column.as_deref();
        let mut layout_rows: Vec<RowSpec> = Vec::new();
        let mut cells_out: Vec<Element<'a, Message, Theme, Renderer>> =
            Vec::new();
        let mut cell_meta: Vec<CellMeta> = Vec::new();

        let push_per_column =
            |layout_rows: &mut Vec<RowSpec>,
             cells_out: &mut Vec<Element<'a, Message, Theme, Renderer>>,
             cell_meta: &mut Vec<CellMeta>,
             layer: CellLayer,
             layer_row: usize,
             group_id: Option<&str>,
             values: &[Cell],
             label_override: Option<&str>| {
                let start = cells_out.len();
                for (col_idx, col) in columns.iter().enumerate() {
                    let cell_value = match (label_override, col_idx) {
                        (Some(label), c)
                            if Some(col.id.as_str()) == stub_id =>
                        {
                            // For summary rows in tables with a stub
                            // column, the row's label is rendered in
                            // the stub column; the positional cells
                            // shift to the non-stub columns. We handle
                            // this by routing the label into the stub
                            // and using `c-1` indexing for values, but
                            // simpler: callers pass full positional
                            // vecs, so we just inject the label here.
                            let _ = c;
                            Cell::Text(label.to_string())
                        }
                        _ => {
                            values.get(col_idx).cloned().unwrap_or(Cell::Empty)
                        }
                    };

                    let coord = CellCoord {
                        layer,
                        row: layer_row,
                        column: Some(col.id.as_str()),
                        group: group_id,
                    };

                    let resolved_style =
                        resolve_style(&tab_styles, &coord, stub_id);
                    let formatter =
                        resolve_formatter(&formatters, &coord, stub_id);

                    let rendered =
                        render_cell(&cell_value, col.kind, formatter.as_ref());
                    let element = build_text_element::<Message, Theme, Renderer>(
                        rendered,
                        col,
                        &resolved_style,
                    );

                    cells_out.push(element);
                    cell_meta.push(CellMeta {
                        style: resolved_style,
                        align: col
                            .align
                            .unwrap_or_else(|| col.kind.default_align()),
                        coord: OwnedCellCoord {
                            layer,
                            row: layer_row,
                            column: Some(col.id.clone()),
                            group: group_id.map(str::to_owned),
                        },
                    });
                }
                let end = cells_out.len();
                layout_rows.push(RowSpec::PerColumn {
                    cell_range: start..end,
                    layer,
                });
            };

        let push_spanned = |layout_rows: &mut Vec<RowSpec>,
                            cells_out: &mut Vec<
            Element<'a, Message, Theme, Renderer>,
        >,
                            cell_meta: &mut Vec<CellMeta>,
                            layer: CellLayer,
                            layer_row: usize,
                            group_id: Option<&str>,
                            text_value: String| {
            let coord = CellCoord {
                layer,
                row: layer_row,
                column: None,
                group: group_id,
            };
            let resolved_style = resolve_style(&tab_styles, &coord, stub_id);

            let element = build_spanned_text_element::<Message, Theme, Renderer>(
                text_value,
                &resolved_style,
                layer,
            );

            let idx = cells_out.len();
            cells_out.push(element);
            cell_meta.push(CellMeta {
                style: resolved_style,
                align: alignment::Horizontal::Left,
                coord: OwnedCellCoord {
                    layer,
                    row: layer_row,
                    column: None,
                    group: group_id.map(str::to_owned),
                },
            });
            layout_rows.push(RowSpec::Spanned {
                cell_index: idx,
                layer,
            });
        };

        if let Some(text) = &title {
            push_spanned(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::Title,
                0,
                None,
                text.clone(),
            );
        }
        if let Some(text) = &subtitle {
            push_spanned(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::Subtitle,
                0,
                None,
                text.clone(),
            );
        }
        if let Some(text) = &units_caption {
            push_spanned(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::UnitsCaption,
                0,
                None,
                text.clone(),
            );
        }

        if !columns.is_empty() {
            let labels: Vec<Cell> = columns
                .iter()
                .map(|col| Cell::Text(col.label.clone()))
                .collect();
            push_per_column(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::ColumnLabels,
                0,
                None,
                &labels,
                None,
            );
        }

        let sticky_block_row_count = layout_rows.len();

        let body_order: Vec<(Option<String>, usize)> =
            if row_groups.is_empty() {
                (0..rows.len()).map(|i| (None, i)).collect()
            } else {
                row_groups
                    .iter()
                    .flat_map(|group| {
                        group.row_indices.iter().map(move |&row_idx| {
                            (Some(group.id.clone()), row_idx)
                        })
                    })
                    .collect()
            };

        let mut last_group: Option<String> = None;
        let mut group_label_index = 0usize;
        let mut summary_layer_row = 0usize;

        for (body_layer_row, (group_id, original_row_idx)) in
            body_order.iter().enumerate()
        {
            if group_id.as_ref() != last_group.as_ref() {
                if let Some(group_id_str) = group_id.as_deref()
                    && let Some(group) =
                        row_groups.iter().find(|g| g.id == group_id_str)
                    && let Some(label) = &group.label
                {
                    push_spanned(
                        &mut layout_rows,
                        &mut cells_out,
                        &mut cell_meta,
                        CellLayer::RowGroupLabels,
                        group_label_index,
                        Some(group_id_str),
                        label.clone(),
                    );
                    group_label_index += 1;
                }
                last_group = group_id.clone();
            }

            let row_cells =
                rows.get(*original_row_idx).cloned().unwrap_or_default();
            push_per_column(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::Body,
                body_layer_row,
                group_id.as_deref(),
                &row_cells,
                None,
            );

            // Emit group-summary rows that belong here, when this is
            // the last row in the current group.
            let is_last_in_group = match group_id {
                Some(gid) => {
                    let next_in_same_group = body_order
                        .iter()
                        .skip_while(|(g, idx)| {
                            !(g.as_ref() == Some(gid)
                                && idx == original_row_idx)
                        })
                        .nth(1)
                        .map(|(g, _)| g.as_ref() == Some(gid))
                        .unwrap_or(false);
                    !next_in_same_group
                }
                None => false,
            };

            if is_last_in_group && let Some(gid) = group_id.as_deref() {
                for summary in group_summary_rows
                    .iter()
                    .filter(|s| s.group_id.as_deref() == Some(gid))
                {
                    push_per_column(
                        &mut layout_rows,
                        &mut cells_out,
                        &mut cell_meta,
                        CellLayer::Summary,
                        summary_layer_row,
                        Some(gid),
                        &summary.cells,
                        Some(&summary.label),
                    );
                    summary_layer_row += 1;
                }
            }
        }

        for (i, summary) in grand_summary_rows.iter().enumerate() {
            push_per_column(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::GrandSummary,
                i,
                None,
                &summary.cells,
                Some(&summary.label),
            );
        }

        for (i, note) in source_notes.iter().enumerate() {
            push_spanned(
                &mut layout_rows,
                &mut cells_out,
                &mut cell_meta,
                CellLayer::SourceNotes,
                i,
                None,
                note.clone(),
            );
        }

        Materialized {
            columns,
            cells: cells_out,
            cell_meta,
            layout_rows,
            sticky_block_row_count,
            stub_column,
            on_press,
            width,
            padding_x,
            padding_y,
            outer_padding_x,
            separator_x,
            separator_y,
            sticky_header,
            class,
        }
    }
}

fn resolve_style(
    tab_styles: &[(Selector, CellStyle)],
    coord: &CellCoord<'_>,
    stub_id: Option<&str>,
) -> CellStyle {
    let mut merged = CellStyle::default();
    for (selector, style) in tab_styles {
        if selector.matches(coord, stub_id) {
            merged.merge(style);
        }
    }
    merged
}

fn resolve_formatter(
    formatters: &[(Selector, Formatter)],
    coord: &CellCoord<'_>,
    stub_id: Option<&str>,
) -> Option<Formatter> {
    let mut latest: Option<Formatter> = None;
    for (selector, formatter) in formatters {
        if selector.matches(coord, stub_id) {
            latest = Some(Arc::clone(formatter));
        }
    }
    latest
}

fn render_cell(
    cell: &Cell,
    kind: ColumnKind,
    formatter: Option<&Formatter>,
) -> String {
    match (kind, cell, formatter) {
        (ColumnKind::Numeric, _, Some(f)) => f(cell),
        (ColumnKind::Numeric, Cell::Number(n), None) => format!("{n}"),
        (ColumnKind::Numeric, Cell::Text(s), None) => s.clone(),
        (ColumnKind::Numeric, Cell::Empty, None) => {
            fmt::EMPTY_GLYPH.to_string()
        }
        (ColumnKind::Text, Cell::Text(s), _) => s.clone(),
        (ColumnKind::Text, Cell::Number(_), Some(f)) => f(cell),
        (ColumnKind::Text, Cell::Number(n), None) => format!("{n}"),
        (ColumnKind::Text, Cell::Empty, _) => String::new(),
    }
}

fn apply_text_transform(s: String, t: Option<TextTransform>) -> String {
    match t {
        Some(TextTransform::Uppercase) => s.to_uppercase(),
        Some(TextTransform::Lowercase) => s.to_lowercase(),
        Some(TextTransform::None) | None => s,
    }
}

fn build_text_element<'a, Message, Theme, Renderer>(
    rendered: String,
    column: &Column,
    style: &CellStyle,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: text::Catalog + 'a,
    <Theme as text::Catalog>::Class<'a>: From<text::StyleFn<'a, Theme>>,
    Renderer: core::text::Renderer<Font = core::Font> + 'a,
{
    let transformed = apply_text_transform(
        rendered,
        style.text.as_ref().and_then(|t| t.transform),
    );
    let mut t = text::Text::<'a, Theme, Renderer>::new(transformed);
    let column_align =
        column.align.unwrap_or_else(|| column.kind.default_align());
    let align = style
        .text
        .as_ref()
        .and_then(|s| s.align)
        .unwrap_or(column_align);
    t = t.align_x(align);
    t = apply_text_style::<Theme, Renderer>(t, style);
    Element::new(t)
}

fn build_spanned_text_element<'a, Message, Theme, Renderer>(
    rendered: String,
    style: &CellStyle,
    layer: CellLayer,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: text::Catalog + 'a,
    <Theme as text::Catalog>::Class<'a>: From<text::StyleFn<'a, Theme>>,
    Renderer: core::text::Renderer<Font = core::Font> + 'a,
{
    let transformed = apply_text_transform(
        rendered,
        style.text.as_ref().and_then(|t| t.transform),
    );
    let mut t = text::Text::<'a, Theme, Renderer>::new(transformed);
    let default_align = match layer {
        CellLayer::Title | CellLayer::Subtitle | CellLayer::UnitsCaption => {
            alignment::Horizontal::Left
        }
        CellLayer::RowGroupLabels => alignment::Horizontal::Left,
        CellLayer::SourceNotes => alignment::Horizontal::Left,
        CellLayer::Body
        | CellLayer::ColumnLabels
        | CellLayer::Stub
        | CellLayer::Summary
        | CellLayer::GrandSummary => alignment::Horizontal::Left,
    };
    let align = style
        .text
        .as_ref()
        .and_then(|s| s.align)
        .unwrap_or(default_align);
    t = t.align_x(align);
    t = apply_text_style::<Theme, Renderer>(t, style);
    Element::new(t)
}

fn apply_text_style<'a, Theme, Renderer>(
    mut t: text::Text<'a, Theme, Renderer>,
    style: &CellStyle,
) -> text::Text<'a, Theme, Renderer>
where
    Theme: text::Catalog + 'a,
    <Theme as text::Catalog>::Class<'a>: From<text::StyleFn<'a, Theme>>,
    Renderer: core::text::Renderer<Font = core::Font> + 'a,
{
    if let Some(ts) = &style.text {
        if let Some(size) = ts.size {
            t = t.size(size);
        }
        if let Some(weight) = ts.weight {
            t = t.weight(weight);
        }
        if let Some(family) = ts.family {
            let mut font = core::Font::DEFAULT;
            font.family = family;
            if let Some(weight) = ts.weight {
                font.weight = weight;
            }
            if let Some(style_) = ts.style {
                font.style = style_;
            }
            t = t.font(font);
        } else if ts.style.is_some() {
            let mut font = core::Font::DEFAULT;
            if let Some(style_) = ts.style {
                font.style = style_;
            }
            if let Some(weight) = ts.weight {
                font.weight = weight;
            }
            t = t.font(font);
        }
        if let Some(color) = ts.color {
            t = t.color(color);
        }
        if let Some(letter_spacing) = ts.letter_spacing {
            t = t.letter_spacing(Em::from(letter_spacing));
        }
        if !ts.features.is_empty() {
            t = t.font_features(ts.features.clone());
        }
    }
    t
}

#[derive(Clone)]
enum RowSpec {
    PerColumn {
        cell_range: std::ops::Range<usize>,
        layer: CellLayer,
    },
    Spanned {
        cell_index: usize,
        layer: CellLayer,
    },
}

impl RowSpec {
    fn layer(&self) -> CellLayer {
        match self {
            RowSpec::PerColumn { layer, .. }
            | RowSpec::Spanned { layer, .. } => *layer,
        }
    }
}

struct CellMeta {
    style: CellStyle,
    align: alignment::Horizontal,
    /// Owned coord rebuilt into a [`CellCoord`] at click-dispatch /
    /// hover time so handlers can be matched against registered
    /// selectors without re-walking the source data.
    coord: OwnedCellCoord,
}

struct Materialized<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    columns: Vec<Column>,
    cells: Vec<Element<'a, Message, Theme, Renderer>>,
    cell_meta: Vec<CellMeta>,
    layout_rows: Vec<RowSpec>,
    sticky_block_row_count: usize,
    /// Stub-column id (if any), needed at hit-test time to resolve
    /// `cells::stub()` selectors against body cells.
    stub_column: Option<String>,
    /// Click-handler accumulator. Walked in registration order at press
    /// dispatch and hover; first matching selector wins.
    on_press: Vec<(Selector, Handler<'a, Message>)>,
    width: Length,
    padding_x: f32,
    padding_y: f32,
    outer_padding_x: Option<f32>,
    separator_x: f32,
    separator_y: f32,
    sticky_header: bool,
    class: <Theme as Catalog>::Class<'a>,
}

struct Metrics {
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    table_width: f32,
    /// Per-cell stride rectangle (full cell area incl. padding) in
    /// table-relative coords. Populated in `layout()` so `draw()` can
    /// paint backgrounds and borders against the row-stride box rather
    /// than the cell's intrinsic text bounds — without this, cells
    /// whose text wraps to multiple lines while their row-mates stay
    /// single-line would draw chrome at mismatched y positions.
    cell_rects: Vec<Rectangle>,
    /// Modifier-key state, kept fresh by intercepting
    /// `keyboard::Event::ModifiersChanged` in `update()`. Read at
    /// click-dispatch time to populate [`Click::modifiers`].
    modifiers: keyboard::Modifiers,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Materialized<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Metrics>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Metrics {
            column_widths: vec![0.0; self.columns.len()],
            row_heights: vec![0.0; self.layout_rows.len()],
            table_width: 0.0,
            cell_rects: vec![Rectangle::default(); self.cells.len()],
            modifiers: keyboard::Modifiers::default(),
        })
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.cells
            .iter()
            .map(|cell| widget::Tree::new(cell.as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.cells);
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let metrics = tree.state.downcast_mut::<Metrics>();
        let limits = limits.width(self.width).height(Length::Shrink);
        let available = limits.max();

        let n_cols = self.columns.len();
        metrics.column_widths = vec![0.0; n_cols];
        metrics.row_heights = vec![0.0; self.layout_rows.len()];

        // `outer_x` is the inset applied to the leftmost cell's left
        // edge and the rightmost cell's right edge (and to spanned-row
        // content). Defaults to `padding_x` for backwards-compat — set
        // explicitly via `Table::outer_padding_x` to align first/last
        // column text with an outer card title while letting borders
        // and fills run edge-to-edge.
        let outer_x = self.outer_padding_x.unwrap_or(self.padding_x);
        let spacing_x = self.padding_x * 2.0 + self.separator_x;
        let spacing_y = self.padding_y * 2.0 + self.separator_y;

        // Pass 1: measure non-fluid columns by laying out per-column
        // cells at intrinsic Shrink widths. Track max width per column.
        let mut cell_layouts: Vec<layout::Node> =
            vec![layout::Node::default(); self.cells.len()];

        let column_factors: Vec<u16> =
            self.columns.iter().map(|c| c.width.fill_factor()).collect();

        for (row_idx, row) in self.layout_rows.iter().enumerate() {
            if let RowSpec::PerColumn { cell_range, .. } = row {
                for (col_offset, cell_idx) in cell_range.clone().enumerate() {
                    let factor = column_factors[col_offset];
                    if factor != 0 {
                        continue;
                    }
                    let column = &self.columns[col_offset];
                    let limits = layout::Limits::new(
                        Size::ZERO,
                        Size::new(available.width, available.height),
                    )
                    .width(column.width)
                    .height(Length::Shrink);
                    let node = self.cells[cell_idx].as_widget_mut().layout(
                        &mut tree.children[cell_idx],
                        renderer,
                        &limits,
                    );
                    let s = node.size();
                    metrics.column_widths[col_offset] =
                        metrics.column_widths[col_offset].max(s.width);
                    metrics.row_heights[row_idx] =
                        metrics.row_heights[row_idx].max(s.height);
                    cell_layouts[cell_idx] = node;
                }
            }
        }

        // Pass 2: allocate fluid column widths over remaining space.
        let total_factor: u16 = column_factors.iter().sum();
        if total_factor > 0 {
            let consumed: f32 = column_factors
                .iter()
                .enumerate()
                .filter_map(|(i, f)| {
                    (*f == 0).then_some(metrics.column_widths[i])
                })
                .sum();
            let total_spacing =
                spacing_x * n_cols.saturating_sub(1) as f32 + outer_x * 2.0;
            let available_width =
                (available.width - consumed - total_spacing).max(0.0);
            let unit = available_width / total_factor as f32;
            for (i, factor) in column_factors.iter().enumerate() {
                if *factor != 0 {
                    metrics.column_widths[i] = unit * *factor as f32;
                }
            }

            for (row_idx, row) in self.layout_rows.iter().enumerate() {
                if let RowSpec::PerColumn { cell_range, .. } = row {
                    for (col_offset, cell_idx) in cell_range.clone().enumerate()
                    {
                        if column_factors[col_offset] == 0 {
                            continue;
                        }
                        let limits = layout::Limits::new(
                            Size::ZERO,
                            Size::new(
                                metrics.column_widths[col_offset],
                                available.height,
                            ),
                        )
                        .width(Length::Fixed(metrics.column_widths[col_offset]))
                        .height(Length::Shrink);
                        let node = self.cells[cell_idx].as_widget_mut().layout(
                            &mut tree.children[cell_idx],
                            renderer,
                            &limits,
                        );
                        let s = node.size();
                        metrics.row_heights[row_idx] =
                            metrics.row_heights[row_idx].max(s.height);
                        cell_layouts[cell_idx] = node;
                    }
                }
            }
        }

        let table_width = metrics.column_widths.iter().sum::<f32>()
            + spacing_x * n_cols.saturating_sub(1) as f32
            + outer_x * 2.0;
        metrics.table_width = table_width;

        // Spanned rows lay out at full table width (minus outer padding
        // so their text starts at the same x as the first column).
        for (row_idx, row) in self.layout_rows.iter().enumerate() {
            if let RowSpec::Spanned { cell_index, .. } = row {
                let inner_width = (table_width - outer_x * 2.0).max(0.0);
                let limits = layout::Limits::new(
                    Size::ZERO,
                    Size::new(inner_width, available.height),
                )
                .width(Length::Fixed(inner_width))
                .height(Length::Shrink);
                let node = self.cells[*cell_index].as_widget_mut().layout(
                    &mut tree.children[*cell_index],
                    renderer,
                    &limits,
                );
                metrics.row_heights[row_idx] = node.size().height;
                cell_layouts[*cell_index] = node;
            }
        }

        // Position pass: place every cell and record its row-stride
        // rectangle so `draw_cell_chrome` paints fills/borders against
        // the row box rather than the cell's intrinsic text bounds.
        // The leftmost cell's stride extends to x=0 (table edge) and
        // the rightmost cell's stride extends to x=table_width, so
        // chrome runs edge-to-edge — the `outer_x` inset only affects
        // where text content sits within those strides.
        metrics.cell_rects = vec![Rectangle::default(); self.cells.len()];
        let mut y = self.padding_y;
        for (row_idx, row) in self.layout_rows.iter().enumerate() {
            let row_height = metrics.row_heights[row_idx];
            let stride_y = y - self.padding_y;
            let stride_h = row_height + self.padding_y * 2.0;
            match row {
                RowSpec::PerColumn { cell_range, .. } => {
                    let mut x = outer_x;
                    let last = cell_range.len().saturating_sub(1);
                    for (col_offset, cell_idx) in cell_range.clone().enumerate()
                    {
                        let col_width = metrics.column_widths[col_offset];
                        let align = self.cell_meta[cell_idx].align;
                        let pad_left = if col_offset == 0 {
                            outer_x
                        } else {
                            self.padding_x
                        };
                        let pad_right = if col_offset == last {
                            outer_x
                        } else {
                            self.padding_x
                        };
                        let node = &mut cell_layouts[cell_idx];
                        node.move_to_mut(Point::new(x, y));
                        node.align_mut(
                            core::Alignment::from(align),
                            core::Alignment::from(alignment::Vertical::Center),
                            Size::new(col_width, row_height),
                        );
                        metrics.cell_rects[cell_idx] = Rectangle {
                            x: x - pad_left,
                            y: stride_y,
                            width: col_width + pad_left + pad_right,
                            height: stride_h,
                        };
                        x += col_width + spacing_x;
                    }
                }
                RowSpec::Spanned { cell_index, .. } => {
                    let node = &mut cell_layouts[*cell_index];
                    node.move_to_mut(Point::new(outer_x, y));
                    metrics.cell_rects[*cell_index] = Rectangle {
                        x: 0.0,
                        y: stride_y,
                        width: table_width,
                        height: stride_h,
                    };
                }
            }
            y += row_height + spacing_y;
        }

        let total_height = if self.layout_rows.is_empty() {
            0.0
        } else {
            y - spacing_y + self.padding_y
        };

        let intrinsic = limits.resolve(
            self.width,
            Length::Shrink,
            Size::new(table_width, total_height),
        );

        layout::Node::with_children(intrinsic, cell_layouts)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &core::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut core::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Keep modifier state fresh. We don't capture the event — other
        // widgets need to see modifier changes too.
        if let core::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) =
            event
        {
            tree.state.downcast_mut::<Metrics>().modifiers = *m;
        }

        for ((cell, child), cell_layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut().update(
                child,
                event,
                cell_layout,
                cursor,
                renderer,
                shell,
                viewport,
            );
        }

        // Click dispatch. Bail if a child consumed the event (e.g. a
        // nested button) so we don't double-fire, and bail when no
        // handlers are registered so we don't pay for hit-testing on
        // every press in non-clickable tables.
        if self.on_press.is_empty() || shell.is_event_captured() {
            return;
        }

        // v1 fires on left-press only. `Click::button` is plumbed
        // through so right-click / context-menu handlers can be added
        // later without breaking the signature.
        let core::Event::Mouse(mouse::Event::ButtonPressed(
            button @ mouse::Button::Left,
        )) = event
        else {
            return;
        };
        let bounds = layout.bounds();
        if !cursor.is_over(bounds) {
            return;
        }

        let metrics = tree.state.downcast_ref::<Metrics>();
        let stub = self.stub_column.as_deref();

        // Sticky-header edge case: when the sticky strip is active,
        // hit-testing here resolves to the ORIGINAL cell at its scrolled
        // position, not the duplicated sticky cell painted on top.
        // Clicks landing on the sticky strip therefore won't fire unless
        // the original cell is also under the cursor. Acceptable for v1
        // — sticky-header rows are typically column labels, which today
        // are most users' fallback non-interactive content.
        let hit = self.cell_meta.iter().enumerate().find(|(i, _)| {
            cursor.is_over(self.absolute_cell_rect(metrics, bounds, *i))
        });

        if let Some((_, meta)) = hit {
            let coord = meta.coord.as_borrowed();
            if let Some((_, handler)) = self
                .on_press
                .iter()
                .find(|(selector, _)| selector.matches(&coord, stub))
            {
                let click = Click {
                    coord,
                    modifiers: metrics.modifiers,
                    button: *button,
                };
                shell.publish(handler(click));
                shell.capture_event();
            }
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let metrics = tree.state.downcast_ref::<Metrics>();
        let table_style = theme.style(&self.class);

        let sticky_height = self.sticky_strip_height(metrics);
        let shift = if self.sticky_header && sticky_height > 0.0 {
            let raw = (viewport.y - bounds.y).max(0.0);
            let max = (bounds.height - sticky_height).max(0.0);
            raw.min(max)
        } else {
            0.0
        };
        let sticky_active = shift > 0.0;
        let sticky_cell_count = self.sticky_cell_count();

        // Cell backgrounds first. We use the row-stride rect captured
        // during `layout()` so chrome (fills, borders) lines up across
        // a row even when one cell wraps onto more lines than its
        // row-mates — laying out against `cell_layout.bounds()` would
        // tie chrome to each cell's intrinsic text height.
        for i in 0..self.cells.len() {
            if sticky_active && i < sticky_cell_count {
                continue;
            }
            let rect = self.absolute_cell_rect(metrics, bounds, i);
            self.draw_cell_chrome(renderer, &self.cell_meta[i], rect, bounds);
        }

        // Row separators.
        if self.separator_y > 0.0 {
            self.draw_row_separators(renderer, bounds, metrics, table_style);
        }
        if self.separator_x > 0.0 {
            self.draw_column_separators(renderer, bounds, metrics, table_style);
        }

        // Cell content.
        for (i, ((cell, child), cell_layout)) in self
            .cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            if sticky_active && i < sticky_cell_count {
                continue;
            }
            cell.as_widget().draw(
                child,
                renderer,
                theme,
                defaults,
                cell_layout,
                cursor,
                viewport,
            );
        }

        if sticky_active {
            let strip_height = sticky_height;
            let strip_y = bounds.y + shift;
            let strip_rect = Rectangle {
                x: bounds.x,
                y: strip_y,
                width: bounds.width,
                height: strip_height + self.padding_y,
            };

            renderer.with_layer(strip_rect, |renderer| {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x,
                            y: strip_y,
                            width: bounds.width,
                            height: strip_height,
                        },
                        snap: true,
                        ..renderer::Quad::default()
                    },
                    table_style.sticky_background,
                );

                let sticky_viewport = Rectangle {
                    y: viewport.y - shift,
                    ..*viewport
                };

                renderer.with_translation(
                    Vector::new(0.0, shift),
                    |renderer| {
                        for (i, ((cell, child), cell_layout)) in self
                            .cells
                            .iter()
                            .zip(&tree.children)
                            .zip(layout.children())
                            .enumerate()
                        {
                            if i >= sticky_cell_count {
                                break;
                            }
                            let rect =
                                self.absolute_cell_rect(metrics, bounds, i);
                            self.draw_cell_chrome(
                                renderer,
                                &self.cell_meta[i],
                                rect,
                                bounds,
                            );
                            cell.as_widget().draw(
                                child,
                                renderer,
                                theme,
                                defaults,
                                cell_layout,
                                cursor,
                                &sticky_viewport,
                            );
                        }
                    },
                );
            });
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let from_children = self
            .cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((cell, child), cell_layout)| {
                cell.as_widget().mouse_interaction(
                    child,
                    cell_layout,
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default();

        // Only override `None` (the default for non-interactive widgets
        // like `text`) — anything more specific from a child (text
        // caret, grab, etc.) wins so we don't mask a child's affordance.
        // Matches the `mouse_area` pattern.
        if from_children != mouse::Interaction::None || self.on_press.is_empty()
        {
            return from_children;
        }

        let bounds = layout.bounds();
        if !cursor.is_over(bounds) {
            return from_children;
        }

        let metrics = tree.state.downcast_ref::<Metrics>();
        let stub = self.stub_column.as_deref();

        let hit = self.cell_meta.iter().enumerate().find(|(i, _)| {
            cursor.is_over(self.absolute_cell_rect(metrics, bounds, *i))
        });

        if let Some((_, meta)) = hit {
            let coord = meta.coord.as_borrowed();
            if self
                .on_press
                .iter()
                .any(|(selector, _)| selector.matches(&coord, stub))
            {
                return mouse::Interaction::Pointer;
            }
        }

        from_children
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        for ((cell, child), cell_layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut().operate(
                child,
                cell_layout,
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.cells,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> Materialized<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn sticky_cell_count(&self) -> usize {
        let mut total = 0;
        for row in self.layout_rows.iter().take(self.sticky_block_row_count) {
            match row {
                RowSpec::PerColumn { cell_range, .. } => {
                    total += cell_range.len();
                }
                RowSpec::Spanned { .. } => {
                    total += 1;
                }
            }
        }
        total
    }

    fn sticky_strip_height(&self, metrics: &Metrics) -> f32 {
        let spacing_y = self.padding_y * 2.0 + self.separator_y;
        if self.sticky_block_row_count == 0 {
            return 0.0;
        }
        let mut h = 0.0;
        for r in 0..self.sticky_block_row_count {
            h += metrics.row_heights[r];
        }
        h + spacing_y * self.sticky_block_row_count.saturating_sub(1) as f32
            + self.padding_y * 2.0
    }

    fn absolute_cell_rect(
        &self,
        metrics: &Metrics,
        bounds: Rectangle,
        cell_index: usize,
    ) -> Rectangle {
        let r = metrics.cell_rects[cell_index];
        Rectangle {
            x: bounds.x + r.x,
            y: bounds.y + r.y,
            width: r.width,
            height: r.height,
        }
    }

    /// Clamps `rect` to the widget's allocated `bounds`. Layout
    /// reports `min(natural_table_width, parent_max)` as the widget
    /// size when the parent constrains us, but `metrics.cell_rects`
    /// stay at their natural widths — spanned rows in particular fill
    /// `[0, table_width]`. Without this clamp, fills, borders, and
    /// separators paint past the right edge we were actually given,
    /// bleeding into whatever sits next to the table.
    fn clip_to_bounds(rect: Rectangle, bounds: Rectangle) -> Rectangle {
        let x = rect.x.max(bounds.x);
        let y = rect.y.max(bounds.y);
        let right = (rect.x + rect.width).min(bounds.x + bounds.width);
        let bottom = (rect.y + rect.height).min(bounds.y + bounds.height);
        Rectangle {
            x,
            y,
            width: (right - x).max(0.0),
            height: (bottom - y).max(0.0),
        }
    }

    fn draw_cell_chrome(
        &self,
        renderer: &mut Renderer,
        meta: &CellMeta,
        cell_rect: Rectangle,
        bounds: Rectangle,
    ) {
        if let Some(fill) = meta.style.fill {
            // Extend the fill across half the column separator gap on
            // each side so horizontally adjacent filled cells meet at
            // the gridline midpoint when `separator_x > 0`. We
            // intentionally do NOT extend vertically — the table-level
            // `separator_y` is meant to read as a row divider, and
            // bridging it across filled rows would erase boundaries
            // the caller still wants visible.
            let fill_rect = Self::clip_to_bounds(
                Rectangle {
                    x: cell_rect.x - self.separator_x / 2.0,
                    y: cell_rect.y,
                    width: cell_rect.width + self.separator_x,
                    height: cell_rect.height,
                },
                bounds,
            );
            if fill_rect.width > 0.0 && fill_rect.height > 0.0 {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: fill_rect,
                        snap: true,
                        ..renderer::Quad::default()
                    },
                    fill,
                );
            }
        }

        if let Some(border) = meta.style.borders {
            let color = border
                .color
                .map(Background::from)
                .unwrap_or(Background::Color(Color::BLACK));
            let width = border.width.unwrap_or(1.0);
            // Borders draw centered on the cell edge — half inside the
            // cell stride, half in the inter-row / inter-column gap.
            // This is the "border-collapse: collapse" model: a border
            // wider than the surrounding gap fully covers it (so a
            // 1.5px header underline absorbs the 1px `separator_y` gap
            // beneath it without leaving an exposed hairline of base
            // color), and adjacent cells with the same border meet on
            // the same line rather than stacking. With separators
            // suppressed at bordered boundaries, this is what makes
            // the row edge read as one clean line instead of "border
            // + gap".
            let mut draw_border = |rect: Rectangle| {
                let clipped = Self::clip_to_bounds(rect, bounds);
                if clipped.width > 0.0 && clipped.height > 0.0 {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: clipped,
                            snap: true,
                            ..renderer::Quad::default()
                        },
                        color,
                    );
                }
            };
            if border.sides.top {
                draw_border(Rectangle {
                    x: cell_rect.x,
                    y: cell_rect.y - width / 2.0,
                    width: cell_rect.width,
                    height: width,
                });
            }
            if border.sides.bottom {
                draw_border(Rectangle {
                    x: cell_rect.x,
                    y: cell_rect.y + cell_rect.height - width / 2.0,
                    width: cell_rect.width,
                    height: width,
                });
            }
            if border.sides.left {
                draw_border(Rectangle {
                    x: cell_rect.x - width / 2.0,
                    y: cell_rect.y,
                    width,
                    height: cell_rect.height,
                });
            }
            if border.sides.right {
                draw_border(Rectangle {
                    x: cell_rect.x + cell_rect.width - width / 2.0,
                    y: cell_rect.y,
                    width,
                    height: cell_rect.height,
                });
            }
        }
    }

    fn draw_row_separators(
        &self,
        renderer: &mut Renderer,
        bounds: Rectangle,
        metrics: &Metrics,
        style: Style,
    ) {
        if self.layout_rows.len() < 2 {
            return;
        }
        let mut y = self.padding_y;
        for r in 0..self.layout_rows.len() - 1 {
            y += metrics.row_heights[r] + self.padding_y;
            let above = self.layout_rows[r].layer();
            let below = self.layout_rows[r + 1].layer();
            // Title-block rows shouldn't get hairline separators
            // between them — the title / subtitle / units caption
            // visually belong to one masthead and lines between them
            // read as cell borders the caller didn't ask for. Callers
            // who want a divider above the column labels can opt in
            // via a `Sides::top()` border on `cells::column_labels()`.
            let in_title_block = |layer: CellLayer| {
                matches!(
                    layer,
                    CellLayer::Title
                        | CellLayer::Subtitle
                        | CellLayer::UnitsCaption
                )
            };
            if in_title_block(above) || in_title_block(below) {
                y += self.separator_y + self.padding_y;
                continue;
            }
            // Suppress the separator when adjacent cells already own
            // the boundary via explicit per-cell borders. Without this
            // a `Sides::bottom()` on the row above (or `Sides::top()`
            // on the row below) doubles up with the table-level
            // separator and reads as a thicker, two-tone line.
            if self.row_has_bottom_border(r) || self.row_has_top_border(r + 1) {
                y += self.separator_y + self.padding_y;
                continue;
            }
            let sep = Self::clip_to_bounds(
                Rectangle {
                    x: bounds.x,
                    y: bounds.y + y,
                    width: bounds.width,
                    height: self.separator_y,
                },
                bounds,
            );
            if sep.width > 0.0 && sep.height > 0.0 {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: sep,
                        snap: true,
                        ..renderer::Quad::default()
                    },
                    style.separator_y,
                );
            }
            y += self.separator_y + self.padding_y;
        }
    }

    fn row_has_bottom_border(&self, row_idx: usize) -> bool {
        self.row_cells(row_idx).any(|i| {
            self.cell_meta[i]
                .style
                .borders
                .as_ref()
                .map(|b| b.sides.bottom)
                .unwrap_or(false)
        })
    }

    fn row_has_top_border(&self, row_idx: usize) -> bool {
        self.row_cells(row_idx).any(|i| {
            self.cell_meta[i]
                .style
                .borders
                .as_ref()
                .map(|b| b.sides.top)
                .unwrap_or(false)
        })
    }

    fn row_cells(
        &self,
        row_idx: usize,
    ) -> Box<dyn Iterator<Item = usize> + '_> {
        match &self.layout_rows[row_idx] {
            RowSpec::PerColumn { cell_range, .. } => {
                Box::new(cell_range.clone())
            }
            RowSpec::Spanned { cell_index, .. } => {
                Box::new(std::iter::once(*cell_index))
            }
        }
    }

    fn draw_column_separators(
        &self,
        renderer: &mut Renderer,
        bounds: Rectangle,
        metrics: &Metrics,
        style: Style,
    ) {
        if metrics.column_widths.len() < 2 {
            return;
        }
        let outer_x = self.outer_padding_x.unwrap_or(self.padding_x);
        let mut x = outer_x;
        for c in 0..metrics.column_widths.len() - 1 {
            x += metrics.column_widths[c] + self.padding_x;
            let sep = Self::clip_to_bounds(
                Rectangle {
                    x: bounds.x + x,
                    y: bounds.y,
                    width: self.separator_x,
                    height: bounds.height,
                },
                bounds,
            );
            if sep.width <= 0.0 || sep.height <= 0.0 {
                x += self.separator_x + self.padding_x;
                continue;
            }
            renderer.fill_quad(
                renderer::Quad {
                    bounds: sep,
                    snap: true,
                    ..renderer::Quad::default()
                },
                style.separator_x,
            );
            x += self.separator_x + self.padding_x;
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Table<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + text::Catalog + 'a,
    <Theme as text::Catalog>::Class<'a>: From<text::StyleFn<'a, Theme>>,
    Renderer: core::text::Renderer<Font = core::Font> + 'a,
{
    fn from(table: Table<'a, Message, Theme, Renderer>) -> Self {
        Element::new(table.build())
    }
}
