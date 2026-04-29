//! `gt::Table` — selector-driven `on_press` handlers.
//!
//! A small country-stats table demonstrating that `on_press` handlers
//! accumulate and dispatch first-match-wins: clicks on the `country`
//! column drill down via the specific handler, while clicks on any
//! other body cell fall through to the broad handler. The press
//! readout below also shows the modifier state plumbed via
//! [`gt::Click::modifiers`].
//!
//! Run with: `cargo run --example gt_clickable`

use iced::widget::{center, column, text};
use iced::{Element, Theme};

use sweeten::widget::gt;
use sweeten::widget::gt::{Cell, Column, cells};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(|_: &App| Theme::Light)
        .title("sweeten • gt::Table on_press")
        .run()
}

#[derive(Default)]
struct App {
    last_action: String,
}

#[derive(Debug, Clone)]
enum Message {
    DrillCountry {
        row: usize,
        modifiers: String,
    },
    SelectCell {
        row: usize,
        column: String,
        modifiers: String,
    },
}

impl App {
    fn new() -> Self {
        Self {
            last_action: "Click any cell.".to_string(),
        }
    }

    fn update(&mut self, message: Message) {
        self.last_action = match message {
            Message::DrillCountry { row, modifiers } => {
                let name = COUNTRIES[row].0;
                format!("DrillCountry: row={row} ({name}){modifiers}")
            }
            Message::SelectCell {
                row,
                column,
                modifiers,
            } => {
                let name = COUNTRIES[row].0;
                format!(
                    "SelectCell: row={row} ({name}), column={column}{modifiers}"
                )
            }
        };
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

        let table = gt::Table::new(columns, rows)
            .title("Click handlers · first-match-wins")
            .subtitle("Country column drills down; other body cells select.")
            .stub_column("country")
            .padding_x(12.0)
            .padding_y(6.0)
            .separator_y(1.0)
            // Specific handler — registered FIRST so it wins on the
            // country column.
            .on_press(cells::body().columns(["country"]), |c| {
                Message::DrillCountry {
                    row: c.coord.row,
                    modifiers: fmt_mods(c.modifiers),
                }
            })
            // Broad fallback — fires on any other body cell.
            .on_press(cells::body(), |c| Message::SelectCell {
                row: c.coord.row,
                column: c.coord.column.unwrap_or("").to_owned(),
                modifiers: fmt_mods(c.modifiers),
            })
            .fmt(cells::body(), gt::decimal(1));

        let readout = text(&self.last_action).size(14);

        center(column![table, readout].spacing(20))
            .padding(20)
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
