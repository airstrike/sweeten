//! `gt::Table` — selector-driven `on_press` handlers.
//!
//! Country-stats table demonstrating:
//!
//! - `on_press` accumulation with first-match-wins dispatch — clicking
//!   the `country` column drills down via the specific handler;
//!   clicking any other body cell falls through to the broad handler
//!   that toggles row selection.
//! - Selection state lives in the user's `App`. The table reflects it
//!   via the existing `tab_style + cells::body().rows(predicate)`
//!   primitives — no `gt`-side selection API needed.
//! - `Click::modifiers` plumbed through to the message readout.
//!
//! Run with: `cargo run --example gt_clickable`

use std::collections::BTreeSet;

use iced::widget::{center, column, container, text};
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
    selected: BTreeSet<usize>,
    last_action: String,
}

#[derive(Debug, Clone)]
enum Message {
    DrillCountry { row: usize, modifiers: String },
    ToggleRow { row: usize, modifiers: String },
}

impl App {
    fn new() -> Self {
        Self {
            selected: BTreeSet::new(),
            last_action: "Click a country to drill; click any other cell \
                          to toggle row selection."
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
            Message::ToggleRow { row, modifiers } => {
                if !self.selected.insert(row) {
                    self.selected.remove(&row);
                }
                let name = COUNTRIES[row].0;
                let verb = if self.selected.contains(&row) {
                    "selected"
                } else {
                    "deselected"
                };
                self.last_action = format!(
                    "{verb} {name} (row {row}){modifiers} — \
                     selection: {:?}",
                    self.selected
                );
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

        // Cloned into the predicate so `Selector::rows`' `'static`
        // bound is satisfied. One small clone per `view` call.
        let selected = self.selected.clone();
        let selected_style = CellStyle {
            fill: Some(Background::Color(color!(0xb3d4fc))),
            ..CellStyle::default()
        };

        let table = gt::Table::new(columns, rows)
            .title("Click handlers · first-match-wins")
            .subtitle(
                "Country drills; other cells toggle row selection \
                 (highlighted via tab_style).",
            )
            .stub_column("country")
            .padding_x(12.0)
            .padding_y(6.0)
            .separator_y(1.0)
            // Highlight rows whose index is in the selection set. Pure
            // existing-API: `tab_style` + a row predicate over the body
            // layer. No `gt`-side selection state.
            .tab_style(
                cells::body().rows(move |i| selected.contains(&i)),
                selected_style,
            )
            // Specific handler — registered FIRST, so it wins on the
            // country column.
            .on_press(cells::body().columns(["country"]), |c| {
                Message::DrillCountry {
                    row: c.coord.row,
                    modifiers: fmt_mods(c.modifiers),
                }
            })
            // Broad fallback — any other body cell toggles selection.
            .on_press(cells::body(), |c| Message::ToggleRow {
                row: c.coord.row,
                modifiers: fmt_mods(c.modifiers),
            })
            .fmt(cells::body(), gt::decimal(1));

        let readout = text(&self.last_action).size(14);

        center(
            column![container(table).style(container::bordered_box), readout]
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
