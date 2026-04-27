//! Display tables.
//!
//! This is a sweetened version of `iced`'s [`Table`] where column headers
//! are optional: pass `None` to [`column()`] for a headerless column, and
//! if every column is headerless the header row is skipped entirely.
//!
//! [`Table`]: https://docs.iced.rs/iced/widget/table/index.html
use crate::core;
use crate::core::alignment;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget;
use crate::core::{
    Alignment, Background, Element, Layout, Length, Pixels, Rectangle, Size,
    Vector, Widget,
};

/// Creates a new [`Table`] with the given columns and rows.
///
/// Columns can be created using the [`column()`] function, while rows can be any
/// iterator over some data type `T`.
pub fn table<'a, 'b, T, Message, Theme, Renderer>(
    columns: impl IntoIterator<Item = Column<'a, 'b, T, Message, Theme, Renderer>>,
    rows: impl IntoIterator<Item = T>,
) -> Table<'a, Message, Theme, Renderer>
where
    T: Clone,
    Message: 'a,
    Theme: Catalog,
    Renderer: core::Renderer,
{
    Table::new(columns, rows)
}

/// Creates a new [`Column`] with the given optional header and view function.
///
/// Pass `Some(element)` to give the column a header, or `None` for a
/// headerless column. If every column in a [`Table`] is headerless, the
/// header row is omitted from the layout.
///
/// The view function will be called for each row in a [`Table`] and it must
/// produce the resulting contents of a cell.
pub fn column<'a, 'b, T, E, Message, Theme, Renderer>(
    header: Option<Element<'a, Message, Theme, Renderer>>,
    view: impl Fn(T) -> E + 'b,
) -> Column<'a, 'b, T, Message, Theme, Renderer>
where
    T: 'a,
    E: Into<Element<'a, Message, Theme, Renderer>>,
{
    Column {
        header,
        view: Box::new(move |data| view(data).into()),
        width: Length::Shrink,
        align_x: alignment::Horizontal::Left,
        align_y: alignment::Vertical::Top,
    }
}

/// A grid-like visual representation of data distributed in columns and rows.
pub struct Table<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer>
where
    Theme: Catalog,
{
    columns: Vec<Column_>,
    cells: Vec<Element<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
    padding_x: f32,
    padding_y: f32,
    separator_x: f32,
    separator_y: f32,
    border: f32,
    header_underline_height: Option<f32>,
    has_header: bool,
    sticky_header: bool,
    class: Theme::Class<'a>,
}

struct Column_ {
    width: Length,
    align_x: alignment::Horizontal,
    align_y: alignment::Vertical,
    fill_width: bool,
    fill_height: bool,
}

impl<'a, Message, Theme, Renderer> Table<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    /// Creates a new [`Table`] with the given columns and rows.
    ///
    /// Columns can be created using the [`column()`] function, while rows can be any
    /// iterator over some data type `T`.
    pub fn new<'b, T>(
        columns: impl IntoIterator<
            Item = Column<'a, 'b, T, Message, Theme, Renderer>,
        >,
        rows: impl IntoIterator<Item = T>,
    ) -> Self
    where
        T: Clone,
        Message: 'a,
    {
        let columns = columns.into_iter();
        let rows = rows.into_iter();

        let mut width = Length::Shrink;
        let mut height = Length::Shrink;

        let mut cells = Vec::with_capacity(
            columns.size_hint().0 * (1 + rows.size_hint().0),
        );

        let mut headers: Vec<Option<Element<'a, Message, Theme, Renderer>>> =
            Vec::with_capacity(columns.size_hint().0);

        let (columns, views): (Vec<_>, Vec<_>) = columns
            .map(|column| {
                width = width.enclose(column.width);

                headers.push(column.header);

                (
                    Column_ {
                        width: column.width,
                        align_x: column.align_x,
                        align_y: column.align_y,
                        fill_width: false,
                        fill_height: false,
                    },
                    column.view,
                )
            })
            .collect();

        // If every column is headerless, skip the header row entirely so
        // the table starts with data at row 0. Otherwise, render the
        // header row and fill any `None` slots with a zero-sized Space so
        // the grid stays rectangular.
        let has_header = headers.iter().any(Option::is_some);

        if has_header {
            for header in headers {
                cells.push(
                    header.unwrap_or_else(|| iced_widget::Space::new().into()),
                );
            }
        }

        for row in rows {
            for view in &views {
                let cell = view(row.clone());
                let size_hint = cell.as_widget().size_hint();

                height = height.enclose(size_hint.height);

                cells.push(cell);
            }
        }

        Self {
            columns,
            cells,
            width,
            height,
            padding_x: 10.0,
            padding_y: 5.0,
            separator_x: 1.0,
            separator_y: 1.0,
            border: 0.0,
            header_underline_height: None,
            has_header,
            sticky_header: false,
            class: Theme::default(),
        }
    }

    /// Sets the width of the [`Table`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the padding of the cells of the [`Table`].
    pub fn padding(self, padding: impl Into<Pixels>) -> Self {
        let padding = padding.into();

        self.padding_x(padding).padding_y(padding)
    }

    /// Sets the horizontal padding of the cells of the [`Table`].
    pub fn padding_x(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding_x = padding.into().0;
        self
    }

    /// Sets the vertical padding of the cells of the [`Table`].
    pub fn padding_y(mut self, padding: impl Into<Pixels>) -> Self {
        self.padding_y = padding.into().0;
        self
    }

    /// Sets the thickness of the line separator between the cells of the [`Table`].
    pub fn separator(self, separator: impl Into<Pixels>) -> Self {
        let separator = separator.into();

        self.separator_x(separator).separator_y(separator)
    }

    /// Sets the thickness of the horizontal line separator between the cells of the [`Table`].
    pub fn separator_x(mut self, separator: impl Into<Pixels>) -> Self {
        self.separator_x = separator.into().0;
        self
    }

    /// Sets the thickness of the vertical line separator between the cells of the [`Table`].
    pub fn separator_y(mut self, separator: impl Into<Pixels>) -> Self {
        self.separator_y = separator.into().0;
        self
    }

    /// Sets the thickness of the outline drawn around the entire [`Table`].
    ///
    /// The border is drawn at the edges of the table bounds, inside the
    /// outer padding (the same way cell separators live inside padding
    /// space). Setting it to `0.0` — the default — disables the outline.
    pub fn border(mut self, border: impl Into<Pixels>) -> Self {
        self.border = border.into().0;
        self
    }

    /// Sets the thickness of the underline drawn directly below the
    /// header row, replacing the regular horizontal separator at that
    /// boundary.
    ///
    /// The underline color is controlled by [`Style::header_underline`];
    /// when that is `None`, the regular [`Style::separator_y`] color is
    /// used at the new thickness. Conversely, leaving the height unset
    /// while a non-default [`Style::header_underline`] color is
    /// configured will draw the underline at [`Table::separator_y`]
    /// thickness.
    ///
    /// Has no effect on tables built entirely from headerless columns.
    pub fn header_underline_height(
        mut self,
        height: impl Into<Pixels>,
    ) -> Self {
        self.header_underline_height = Some(height.into().0);
        self
    }

    /// Makes the header row stay pinned to the top of the visible area
    /// as the [`Table`] is scrolled inside a parent scrollable.
    ///
    /// When enabled and the table has a header row, the header is drawn
    /// translated so it tracks `viewport.y`, with a background fill
    /// ([`Style::sticky_background`]) so scrolling data rows don't show
    /// through. The sticky translation is capped so the header never
    /// floats outside the table's vertical bounds — once the table is
    /// scrolled out the bottom, the header scrolls out with it.
    ///
    /// Has no effect on tables built entirely from headerless columns.
    pub fn sticky_header(mut self, sticky: bool) -> Self {
        self.sticky_header = sticky;
        self
    }

    /// Sets the style of the [`Table`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Sets the style class of the [`Table`].
    #[must_use]
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }
}

struct Metrics {
    columns: Vec<f32>,
    rows: Vec<f32>,
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Table<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Metrics>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Metrics {
            columns: Vec::new(),
            rows: Vec::new(),
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
        let columns = self.columns.len();
        let rows = self.cells.len() / columns;

        let limits = limits.width(self.width).height(self.height);
        let available = limits.max();
        let table_fluid = self.width.fluid();

        let mut cells = Vec::with_capacity(self.cells.len());
        cells.resize(self.cells.len(), layout::Node::default());

        metrics.columns = vec![0.0; self.columns.len()];
        metrics.rows = vec![0.0; rows];

        let mut column_factors = vec![0; self.columns.len()];
        let mut total_row_factors = 0;
        let mut total_fluid_height = 0.0;
        let mut row_factor = 0;

        let spacing_x = self.padding_x * 2.0 + self.separator_x;
        let spacing_y = self.padding_y * 2.0 + self.separator_y;
        // The header→body gap can be thicker than other row gaps when an
        // underline is configured (see `header_underline_height`). Falls
        // back to `spacing_y` when no underline is requested or when the
        // table has no header at all.
        let header_spacing_y = if self.has_header {
            self.padding_y * 2.0
                + self.header_underline_height.unwrap_or(self.separator_y)
        } else {
            spacing_y
        };
        let row_spacing = |row: usize| {
            if row == 1 {
                header_spacing_y
            } else {
                spacing_y
            }
        };

        // FIRST PASS
        // Lay out non-fluid cells
        let mut x = self.padding_x;
        let mut y = self.padding_y;

        for (i, (cell, state)) in
            self.cells.iter_mut().zip(&mut tree.children).enumerate()
        {
            let row = i / columns;
            let column = i % columns;

            let width = self.columns[column].width;
            let size = cell.as_widget().size();

            if column == 0 {
                x = self.padding_x;

                if row > 0 {
                    y += metrics.rows[row - 1] + row_spacing(row);

                    if row_factor != 0 {
                        total_fluid_height += metrics.rows[row - 1];
                        total_row_factors += row_factor;

                        row_factor = 0;
                    }
                }
            }

            let width_factor = width.fill_factor();

            // Detect fill columns from cell size hints
            if size.width.is_fill() {
                self.columns[column].fill_width = true;
            }
            if size.height.is_fill() {
                self.columns[column].fill_height = true;
            }

            // Skip width-fluid cells for pass 2
            if width_factor != 0 || size.width.is_fill() {
                column_factors[column] =
                    column_factors[column].max(width_factor);

                row_factor = row_factor.max(size.height.fill_factor());

                // Still measure the cell at its intrinsic size so it
                // contributes to `metrics.rows[row]` AND
                // `metrics.columns[column]`. Without this, a row or
                // column made entirely of width-fluid cells (e.g. a
                // table of `container(text).width(Fill).height(Fill)`
                // cells that want their background fill to extend from
                // separator to separator) leaves the corresponding
                // metric at 0. Pass 2 refuses to update row heights for
                // fill-height cells, and a Shrink column of Fill cells
                // reads `metrics.columns[column]` as its pass-2 max
                // width — so without this floor the cell compresses to
                // zero width. Shrink-shrink limits flip the compression
                // flag (see `iced_core::layout::Limits::resolve`) which
                // makes Fill-width/height widgets fall through to their
                // intrinsic size instead of the limits max, giving us
                // true natural dimensions without fighting the fill
                // width/height that pass 3 will later apply.
                let natural_limits = layout::Limits::new(
                    Size::ZERO,
                    Size::new(
                        (available.width - x).max(0.0),
                        (available.height - y).max(0.0),
                    ),
                )
                .width(Length::Shrink)
                .height(Length::Shrink);
                let natural = cell.as_widget_mut().layout(
                    state,
                    renderer,
                    &natural_limits,
                );
                let natural_size = natural.size();
                metrics.rows[row] = metrics.rows[row].max(natural_size.height);
                // Only write the column metric when the column itself is
                // non-fluid (width_factor == 0). Fluid columns have their
                // widths resolved by pass 2's fluid-space allocation
                // (`width_unit * factor`), and overwriting that with the
                // intrinsic width would risk pushing fluid columns past
                // the table's bounds when the text is wider than the
                // allocated share. For non-fluid (Shrink/Fixed) columns
                // whose cells report `size.width.is_fill() == true`,
                // pass 2 reads `metrics.columns[column]` as the cell's
                // max width — so without this write the column would
                // collapse to zero.
                if width_factor == 0 {
                    metrics.columns[column] =
                        metrics.columns[column].max(natural_size.width);
                }

                continue;
            }

            // Lay out cell with compressed height so fill-height cells
            // contribute their intrinsic content height to row metrics
            let limits = layout::Limits::new(
                Size::ZERO,
                Size::new(available.width - x, available.height - y),
            )
            .width(width)
            .height(Length::Shrink);

            let layout = cell.as_widget_mut().layout(state, renderer, &limits);
            let size = limits.resolve(width, Length::Shrink, layout.size());

            metrics.columns[column] = metrics.columns[column].max(size.width);
            metrics.rows[row] = metrics.rows[row].max(size.height);
            cells[i] = layout;

            x += size.width + spacing_x;
        }

        // SECOND PASS
        // Lay out fluid cells, using metrics from the first pass as limits
        let left = Size::new(
            available.width
                - metrics
                    .columns
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| column_factors[*i] == 0)
                    .map(|(_, width)| width)
                    .sum::<f32>(),
            available.height - total_fluid_height,
        );

        let width_unit = (left.width
            - spacing_x * self.columns.len().saturating_sub(1) as f32
            - self.padding_x * 2.0)
            / column_factors.iter().sum::<u16>() as f32;

        let height_unit = (left.height
            - spacing_y * rows.saturating_sub(1) as f32
            - (header_spacing_y - spacing_y)
            - self.padding_y * 2.0)
            / total_row_factors as f32;

        let mut x = self.padding_x;
        let mut y = self.padding_y;

        for (i, (cell, state)) in
            self.cells.iter_mut().zip(&mut tree.children).enumerate()
        {
            let row = i / columns;
            let column = i % columns;

            let size = cell.as_widget().size();

            let width = self.columns[column].width;
            let width_factor = width.fill_factor();
            let height_factor = size.height.fill_factor();

            if column == 0 {
                x = self.padding_x;

                if row > 0 {
                    y += metrics.rows[row - 1] + row_spacing(row);
                }
            }

            if width_factor == 0
                && size.width.fill_factor() == 0
                && size.height.fill_factor() == 0
            {
                continue;
            }

            let max_width = if width_factor == 0 {
                if size.width.is_fill() {
                    metrics.columns[column]
                } else {
                    (available.width - x).max(0.0)
                }
            } else {
                width_unit * width_factor as f32
            };

            let max_height = if height_factor == 0 {
                if size.height.is_fill() {
                    metrics.rows[row]
                } else {
                    (available.height - y).max(0.0)
                }
            } else {
                height_unit * height_factor as f32
            };

            let limits = layout::Limits::new(
                Size::ZERO,
                Size::new(max_width, max_height),
            )
            .width(width);

            let layout = cell.as_widget_mut().layout(state, renderer, &limits);
            let size = limits.resolve(
                if let Length::Fixed(_) = width {
                    width
                } else {
                    table_fluid
                },
                Length::Shrink,
                layout.size(),
            );

            metrics.columns[column] = metrics.columns[column].max(size.width);
            if !(self.columns[column].fill_height && height_factor != 0) {
                metrics.rows[row] = metrics.rows[row].max(size.height);
            }
            cells[i] = layout;

            x += size.width + spacing_x;
        }

        // THIRD PASS
        // Re-layout fill cells with finalized dimensions.
        // Fill cells get the full band (content + padding on both sides)
        // so backgrounds and canvas widgets extend from separator to separator.
        if self.columns.iter().any(|c| c.fill_width || c.fill_height) {
            for (i, (cell, state)) in
                self.cells.iter_mut().zip(&mut tree.children).enumerate()
            {
                let column = i % columns;
                let col = &self.columns[column];

                if !col.fill_width && !col.fill_height {
                    continue;
                }

                let cell_size = cell.as_widget().size();
                let cell_fw = col.fill_width && cell_size.width.is_fill();
                let cell_fh = col.fill_height && cell_size.height.is_fill();

                if !cell_fw && !cell_fh {
                    continue;
                }

                let row = i / columns;
                let fill_width = if cell_fw {
                    metrics.columns[column] + self.padding_x * 2.0
                } else {
                    metrics.columns[column]
                };
                let fill_height = if cell_fh {
                    metrics.rows[row] + self.padding_y * 2.0
                } else {
                    metrics.rows[row]
                };
                let mut limits = layout::Limits::new(
                    Size::ZERO,
                    Size::new(fill_width, fill_height),
                );

                if !cell_fw {
                    limits = limits.width(col.width);
                }

                cells[i] =
                    cell.as_widget_mut().layout(state, renderer, &limits);
            }
        }

        // FOURTH PASS
        // Position each cell
        let mut x = self.padding_x;
        let mut y = self.padding_y;

        for (i, cell) in cells.iter_mut().enumerate() {
            let row = i / columns;
            let column = i % columns;

            if column == 0 {
                x = self.padding_x;

                if row > 0 {
                    y += metrics.rows[row - 1] + row_spacing(row);
                }
            }

            let Column_ {
                align_x, align_y, ..
            } = &self.columns[column];

            let col = &self.columns[column];
            let cell_size = self.cells[i].as_widget().size();
            let cell_fw = col.fill_width && cell_size.width.is_fill();
            let cell_fh = col.fill_height && cell_size.height.is_fill();

            let (cell_x, cell_width) = if cell_fw {
                (
                    x - self.padding_x,
                    metrics.columns[column] + self.padding_x * 2.0,
                )
            } else {
                (x, metrics.columns[column])
            };
            let (cell_y, cell_height) = if cell_fh {
                (y - self.padding_y, metrics.rows[row] + self.padding_y * 2.0)
            } else {
                (y, metrics.rows[row])
            };

            cell.move_to_mut((cell_x, cell_y));
            cell.align_mut(
                Alignment::from(*align_x),
                Alignment::from(*align_y),
                Size::new(cell_width, cell_height),
            );

            x += metrics.columns[column] + spacing_x;
        }

        let intrinsic = limits.resolve(
            self.width,
            self.height,
            Size::new(
                x - spacing_x + self.padding_x,
                y + metrics
                    .rows
                    .last()
                    .copied()
                    .map(|height| height + self.padding_y)
                    .unwrap_or_default(),
            ),
        );

        layout::Node::with_children(intrinsic, cells)
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
        for ((cell, tree), layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut()
                .update(tree, event, layout, cursor, renderer, shell, viewport);
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let metrics = tree.state.downcast_ref::<Metrics>();
        let table_style = theme.style(&self.class);
        let num_columns = self.columns.len();

        // The thickness/color of the line below the header row. Falls
        // back to the regular `separator_y` thickness/color when neither
        // override is set, so a table with no header underline configured
        // draws exactly as before.
        let header_underline_height = if self.has_header {
            self.header_underline_height.unwrap_or(self.separator_y)
        } else {
            self.separator_y
        };
        let header_underline_color = table_style
            .header_underline
            .unwrap_or(table_style.separator_y);

        // How far past the table's top the user has scrolled, in the
        // table's own content coordinates. Inside an iced `scrollable`,
        // `viewport.y` is already adjusted into content space, so this
        // delta is the scroll amount. We cap it at the distance between
        // the header's natural position and the bottom of the table so
        // the sticky header can't float past the last row.
        let shift = if self.sticky_header
            && self.has_header
            && !metrics.rows.is_empty()
        {
            let raw = (viewport.y - bounds.y).max(0.0);
            let header_height = metrics.rows[0];
            let max =
                (bounds.height - header_height - 2.0 * self.padding_y).max(0.0);
            raw.min(max)
        } else {
            0.0
        };
        let sticky_active = shift > 0.0;

        for (i, ((cell, state), cell_layout)) in self
            .cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .enumerate()
        {
            // If the sticky overlay is going to redraw the header row,
            // skip it here so it isn't drawn twice.
            if sticky_active && i < num_columns {
                continue;
            }
            cell.as_widget().draw(
                state,
                renderer,
                theme,
                style,
                cell_layout,
                cursor,
                viewport,
            );
        }

        if self.separator_x > 0.0 {
            let mut x = self.padding_x;

            for width in
                &metrics.columns[..metrics.columns.len().saturating_sub(1)]
            {
                x += width + self.padding_x;

                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: bounds.x + x,
                            y: bounds.y,
                            width: self.separator_x,
                            height: bounds.height,
                        },
                        snap: true,
                        ..renderer::Quad::default()
                    },
                    table_style.separator_x,
                );

                x += self.separator_x + self.padding_x;
            }
        }

        if self.separator_y > 0.0 || header_underline_height > 0.0 {
            let mut y = self.padding_y;

            for (i, height) in metrics.rows
                [..metrics.rows.len().saturating_sub(1)]
                .iter()
                .enumerate()
            {
                y += height + self.padding_y;

                let (sep_height, sep_color) = if i == 0 && self.has_header {
                    (header_underline_height, header_underline_color)
                } else {
                    (self.separator_y, table_style.separator_y)
                };

                if sep_height > 0.0 {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: bounds.y + y,
                                width: bounds.width,
                                height: sep_height,
                            },
                            snap: true,
                            ..renderer::Quad::default()
                        },
                        sep_color,
                    );
                }

                y += sep_height + self.padding_y;
            }
        }

        if sticky_active {
            let header_height = metrics.rows[0];
            let strip_height = header_height + 2.0 * self.padding_y;
            let strip_y = bounds.y + shift;

            let strip_rect = Rectangle {
                x: bounds.x,
                y: strip_y,
                // Include the underline below the strip inside the clip.
                height: strip_height + header_underline_height,
                width: bounds.width,
            };

            // `with_layer` pushes a fresh layer, which is rendered after
            // the main content layer — without this, iced batches all
            // quads below all text within a layer, so our background
            // fill would end up behind the data-row text it's meant to
            // occlude. The clip to `strip_rect` also prevents any
            // overflow from the translated header from bleeding into
            // the rest of the table.
            renderer.with_layer(strip_rect, |renderer| {
                // Opaque background so data rows scrolling past don't
                // show through the translated header cells.
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

                // Vertical separators drawn again inside the sticky
                // strip, since the background fill above just covered
                // the ones belonging to the main draw pass.
                if self.separator_x > 0.0 {
                    let mut x = self.padding_x;

                    for width in &metrics.columns
                        [..metrics.columns.len().saturating_sub(1)]
                    {
                        x += width + self.padding_x;

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: bounds.x + x,
                                    y: strip_y,
                                    width: self.separator_x,
                                    height: strip_height,
                                },
                                snap: true,
                                ..renderer::Quad::default()
                            },
                            table_style.separator_x,
                        );

                        x += self.separator_x + self.padding_x;
                    }
                }

                // Header cells, translated by `shift` so they land at
                // the top of the visible viewport (or against the
                // bottom cap). We also shift the `viewport` we pass
                // down by `-shift` so that children (e.g. `text`) which
                // use it as a clip rectangle (see `core::widget::text`)
                // compute their clip relative to the translated draw
                // position — otherwise text ascenders land above the
                // clip top and get cut off.
                let sticky_viewport = Rectangle {
                    y: viewport.y - shift,
                    ..*viewport
                };
                renderer.with_translation(
                    Vector::new(0.0, shift),
                    |renderer| {
                        for (i, ((cell, state), cell_layout)) in self
                            .cells
                            .iter()
                            .zip(&tree.children)
                            .zip(layout.children())
                            .enumerate()
                        {
                            if i >= num_columns {
                                break;
                            }
                            cell.as_widget().draw(
                                state,
                                renderer,
                                theme,
                                style,
                                cell_layout,
                                cursor,
                                &sticky_viewport,
                            );
                        }
                    },
                );

                // Header underline drawn directly below the sticky strip.
                // Falls back to `separator_y` thickness/color when no
                // underline is configured.
                if header_underline_height > 0.0 {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: strip_y + strip_height,
                                width: bounds.width,
                                height: header_underline_height,
                            },
                            snap: true,
                            ..renderer::Quad::default()
                        },
                        header_underline_color,
                    );
                }

                // Border segments pinned with the sticky strip. The
                // main border pass draws at `bounds.y` / `bounds.x`
                // etc., so the top edge scrolls out of view and the
                // left/right edges get painted over by the background
                // fill inside the strip. Redraw the missing pieces
                // here so the visible table frame stays continuous.
                if self.border > 0.0 {
                    let frame_height = strip_height + header_underline_height;

                    // Top edge pinned at the top of the strip.
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: strip_y,
                                width: bounds.width,
                                height: self.border,
                            },
                            snap: true,
                            ..renderer::Quad::default()
                        },
                        table_style.border,
                    );
                    // Left segment through the strip.
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: strip_y,
                                width: self.border,
                                height: frame_height,
                            },
                            snap: true,
                            ..renderer::Quad::default()
                        },
                        table_style.border,
                    );
                    // Right segment through the strip.
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x: bounds.x + bounds.width - self.border,
                                y: strip_y,
                                width: self.border,
                                height: frame_height,
                            },
                            snap: true,
                            ..renderer::Quad::default()
                        },
                        table_style.border,
                    );
                }
            });
        }

        if self.border > 0.0 {
            // Top edge
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: bounds.y,
                        width: bounds.width,
                        height: self.border,
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                table_style.border,
            );
            // Bottom edge
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: bounds.y + bounds.height - self.border,
                        width: bounds.width,
                        height: self.border,
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                table_style.border,
            );
            // Left edge
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: bounds.y,
                        width: self.border,
                        height: bounds.height,
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                table_style.border,
            );
            // Right edge
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + bounds.width - self.border,
                        y: bounds.y,
                        width: self.border,
                        height: bounds.height,
                    },
                    snap: true,
                    ..renderer::Quad::default()
                },
                table_style.border,
            );
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
        self.cells
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((cell, tree), layout)| {
                cell.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        for ((cell, state), layout) in self
            .cells
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            cell.as_widget_mut()
                .operate(state, layout, renderer, operation);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: core::Vector,
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

impl<'a, Message, Theme, Renderer> From<Table<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: core::Renderer + 'a,
{
    fn from(table: Table<'a, Message, Theme, Renderer>) -> Self {
        Element::new(table)
    }
}

/// A vertical visualization of some data with an optional header.
pub struct Column<
    'a,
    'b,
    T,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> {
    header: Option<Element<'a, Message, Theme, Renderer>>,
    view: Box<dyn Fn(T) -> Element<'a, Message, Theme, Renderer> + 'b>,
    width: Length,
    align_x: alignment::Horizontal,
    align_y: alignment::Vertical,
}

impl<'a, 'b, T, Message, Theme, Renderer>
    Column<'a, 'b, T, Message, Theme, Renderer>
{
    /// Sets the width of the [`Column`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the alignment for the horizontal axis of the [`Column`].
    pub fn align_x(
        mut self,
        alignment: impl Into<alignment::Horizontal>,
    ) -> Self {
        self.align_x = alignment.into();
        self
    }

    /// Sets the alignment for the vertical axis of the [`Column`].
    pub fn align_y(
        mut self,
        alignment: impl Into<alignment::Vertical>,
    ) -> Self {
        self.align_y = alignment.into();
        self
    }

    /// Centers the content of the [`Column`] both horizontally and vertically.
    pub fn center(self) -> Self {
        self.align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
    }
}

/// The appearance of a [`Table`].
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// The background color of the horizontal line separator between cells.
    pub separator_x: Background,
    /// The background color of the vertical line separator between cells.
    pub separator_y: Background,
    /// The background color of the outline drawn around the entire table.
    pub border: Background,
    /// The background fill drawn behind the sticky header row, so that
    /// scrolling data rows don't show through. Only used when
    /// [`Table::sticky_header`] is enabled.
    pub sticky_background: Background,
    /// The background color of the underline drawn directly below the
    /// header row, replacing the regular horizontal separator there.
    /// When `None`, the [`Style::separator_y`] color is used at the
    /// header boundary like every other row. Pair with
    /// [`Table::header_underline_height`] to control its thickness.
    pub header_underline: Option<Background>,
}

/// The theme catalog of a [`Table`].
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`Table`].
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

/// The default style of a [`Table`].
pub fn default(theme: &crate::Theme) -> Style {
    let palette = theme.palette();
    let separator = palette.background.strong.color.into();

    Style {
        separator_x: separator,
        separator_y: separator,
        border: separator,
        sticky_background: palette.background.base.color.into(),
        header_underline: None,
    }
}
