//! `gt::Table` — selector-driven `on_press` handlers with an
//! either-or row/column selection model.
//!
//! Click a body cell (other than the country) to toggle row selection;
//! click any column header to toggle column selection. The two are
//! mutually exclusive: picking one clears the other. Clicking a country
//! body cell still drills (the specific handler wins on that column).
//!
//! Selection lives in user state as a [`Selection`] enum that makes the
//! mutual-exclusion invariant structural — you can't accidentally
//! populate both. The table reflects it via
//! `tab_style + cells::body().rows(...) / .columns(...)` plus
//! `cells::column_labels().columns(...)` so headers tint along with
//! their column.
//!
//! Also demonstrates gt's outer border (`.border(1.0)`) and
//! sticky-header behavior inside a fixed-height `scrollable`.
//!
//! Run with: `cargo run --example gt_clickable`

use std::collections::BTreeSet;

use iced::widget::{center, column, scrollable, text};
use iced::{Background, Element, Theme, color};

use sweeten::widget::gt;
use sweeten::widget::gt::{Cell, CellStyle, Column, cells};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(|_: &App| Theme::Light)
        .title("sweeten • gt::Table on_press")
        .run()
}

#[derive(Default)]
struct App {
    selection: Selection,
    last_action: String,
}

#[derive(Default)]
enum Selection {
    #[default]
    None,
    Rows(BTreeSet<usize>),
    Columns(BTreeSet<String>),
}

impl Selection {
    fn toggle_row(&mut self, row: usize) {
        let mut set = match std::mem::replace(self, Self::None) {
            Self::Rows(s) => s,
            _ => BTreeSet::new(),
        };
        if !set.insert(row) {
            set.remove(&row);
        }
        if !set.is_empty() {
            *self = Self::Rows(set);
        }
    }

    fn toggle_column(&mut self, column: String) {
        let mut set = match std::mem::replace(self, Self::None) {
            Self::Columns(s) => s,
            _ => BTreeSet::new(),
        };
        if !set.remove(&column) {
            set.insert(column);
        }
        if !set.is_empty() {
            *self = Self::Columns(set);
        }
    }

    fn row_set(&self) -> BTreeSet<usize> {
        match self {
            Self::Rows(s) => s.clone(),
            _ => BTreeSet::new(),
        }
    }

    fn column_set(&self) -> BTreeSet<String> {
        match self {
            Self::Columns(s) => s.clone(),
            _ => BTreeSet::new(),
        }
    }

    fn describe(&self) -> String {
        match self {
            Self::None => "no selection".to_string(),
            Self::Rows(s) => format!("rows {s:?}"),
            Self::Columns(s) => format!("columns {s:?}"),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    DrillCountry { row: usize, modifiers: String },
    ToggleRow(usize),
    ToggleColumn(String),
}

impl App {
    fn new() -> Self {
        Self {
            selection: Selection::None,
            last_action: "Click body to select rows; click a header to \
                          select columns. Picking one clears the other."
                .to_string(),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::DrillCountry { row, modifiers } => {
                let name = COUNTRIES[row].0;
                self.last_action =
                    format!("DrillCountry → {name} (row {row}){modifiers}");
            }
            Message::ToggleRow(row) => {
                let name = COUNTRIES[row].0;
                self.selection.toggle_row(row);
                self.last_action = format!("toggled row {row} ({name})");
            }
            Message::ToggleColumn(column) => {
                self.last_action = format!("toggled column \"{column}\"");
                self.selection.toggle_column(column);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let columns = vec![
            Column::text("country", "Country").width(160.0),
            Column::numeric("population", "Population (M)").width(140.0),
            Column::numeric("gdp", "GDP ($B)").width(120.0),
        ];

        let rows: Vec<Vec<Cell>> = COUNTRIES
            .iter()
            .map(|(name, pop, gdp)| {
                vec![Cell::text(*name), Cell::Number(*pop), Cell::Number(*gdp)]
            })
            .collect();

        let row_set = self.selection.row_set();
        let col_set = self.selection.column_set();
        let highlight = CellStyle {
            fill: Some(Background::Color(color!(0xb3d4fc))),
            ..CellStyle::default()
        };

        let table = gt::Table::new(columns, rows)
            .title("Country statistics, 2023")
            .subtitle("Population in millions; GDP in USD billions.")
            .stub_column("country")
            .padding_x(12.0)
            .padding_y(6.0)
            .separator_y(1.0)
            .border(1.0)
            .sticky_header(true)
            // Visualization. Three calls — at most one of `row_set` /
            // `col_set` is non-empty, so only one tints anything.
            .tab_style(
                cells::body().rows(move |i| row_set.contains(&i)),
                highlight.clone(),
            )
            .tab_style(
                cells::body().columns(col_set.clone()),
                highlight.clone(),
            )
            .tab_style(cells::column_labels().columns(col_set), highlight)
            // Specific handler — country body cell drills (first-match
            // wins over the broad body fallback below).
            .on_press(cells::body().columns(["country"]), |c| {
                Message::DrillCountry {
                    row: c.coord.row,
                    modifiers: fmt_mods(c.modifiers),
                }
            })
            // Broad fallback — any other body cell toggles row.
            .on_press(cells::body(), |c| Message::ToggleRow(c.coord.row))
            // Any column header toggles that column.
            .on_press(cells::column_labels(), |c| {
                Message::ToggleColumn(c.coord.column.unwrap_or("").to_owned())
            })
            .fmt(cells::body(), gt::decimal(1));

        let instructions = text(
            "Click body cells to toggle row selection; click headers to \
             toggle column selection. Picking one clears the other. The \
             country column drills (specific handler wins).",
        )
        .size(13);
        let action = text(&self.last_action).size(14);
        let state = text(self.selection.describe()).size(14);

        center(
            column![
                instructions,
                scrollable(table).height(220.0),
                action,
                state
            ]
            .spacing(20.0),
        )
        .padding(20.0)
        .into()
    }
}

fn fmt_mods(modifiers: iced::keyboard::Modifiers) -> String {
    if modifiers.is_empty() {
        String::new()
    } else {
        let mut parts = Vec::new();
        if modifiers.shift() {
            parts.push("Shift");
        }
        if modifiers.control() {
            parts.push("Ctrl");
        }
        if modifiers.alt() {
            parts.push("Alt");
        }
        if modifiers.logo() {
            parts.push("Cmd");
        }
        format!(" [{}]", parts.join("+"))
    }
}

const COUNTRIES: &[(&str, f64, f64)] = &[
    ("United States", 334.9, 27_360.0),
    ("China", 1_410.7, 17_790.0),
    ("Germany", 84.5, 4_456.0),
    ("Japan", 125.4, 4_213.0),
    ("Brazil", 216.4, 2_174.0),
];
