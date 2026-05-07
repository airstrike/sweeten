//! `gt::Table` — financial summary in the grammar-of-tables style.
//!
//! Builds a "Revenue → Gross Profit" table for FY 2026 with monthly
//! columns and an aggregate FY column, exercising selector-based
//! styling and number formatters.
//!
//! Run with: `cargo run --example gt`

use iced::widget::{center, scrollable};
use iced::{Element, Theme, alignment, color, font};

use sweeten::widget::gt;
use sweeten::widget::gt::{
    BorderStyle, Cell, CellStyle, Column, RowGroup, Sides, SummaryRow,
    TextStyle, TextTransform, cells,
};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(|_: &App| Theme::Light)
        .title("sweeten • gt::Table")
        .run()
}

#[derive(Default)]
struct App;

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new() -> Self {
        Self
    }

    fn update(&mut self, _: Message) {}

    fn view(&self) -> Element<'_, Message> {
        let columns = build_columns();
        let rows = build_rows();
        let row_groups = build_row_groups();
        let summary_rows = build_summary_rows();
        let grand_summary_rows = build_grand_summary_rows();

        let neutral_50 = color!(0xfafafa);
        let neutral_100 = color!(0xf5f5f5);
        let neutral_200 = color!(0xe5e5e5);
        let neutral_300 = color!(0xd4d4d4);
        let neutral_500 = color!(0x737373);
        let neutral_900 = color!(0x171717);

        let table = gt::Table::new(columns, rows)
            .title("Financial Summary · Revenue to Gross Profit")
            .subtitle("FY 2026 · Monthly breakdown")
            .units_caption("Figures in $M")
            .stub_column("line_item")
            .row_groups(row_groups)
            .summary_rows(summary_rows)
            .grand_summary_rows(grand_summary_rows)
            .source_notes(vec![
                "Source: internal management accounts, unaudited.".to_string(),
            ])
            .padding_x(12.0)
            .padding_y(6.0)
            .separator_y(1.0)
            .sticky_header(true)
            .animate_on_load()
            .tab_style(
                cells::title(),
                CellStyle {
                    text: Some(TextStyle {
                        size: Some(20.0),
                        weight: Some(font::Weight::Semibold),
                        color: Some(neutral_900),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::subtitle(),
                CellStyle {
                    text: Some(TextStyle {
                        size: Some(13.0),
                        color: Some(neutral_500),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::units_caption(),
                CellStyle {
                    text: Some(TextStyle {
                        size: Some(11.0),
                        color: Some(neutral_500),
                        style: Some(font::Style::Italic),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::column_labels(),
                CellStyle {
                    text: Some(TextStyle {
                        size: Some(11.0),
                        weight: Some(font::Weight::Medium),
                        color: Some(neutral_500),
                        transform: Some(TextTransform::Uppercase),
                        letter_spacing: Some(0.6),
                        ..Default::default()
                    }),
                    borders: Some(BorderStyle {
                        sides: Sides::bottom(),
                        color: Some(neutral_300),
                        width: Some(1.5),
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::row_group_labels(),
                CellStyle {
                    text: Some(TextStyle {
                        size: Some(11.0),
                        weight: Some(font::Weight::Bold),
                        color: Some(neutral_900),
                        transform: Some(TextTransform::Uppercase),
                        letter_spacing: Some(0.4),
                        ..Default::default()
                    }),
                    fill: Some(neutral_100.into()),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::body().rows(|i| INDENTED_BODY_ROWS.contains(&i)),
                CellStyle {
                    text: Some(TextStyle {
                        color: Some(neutral_500),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::body().columns(["fy_2026"]),
                CellStyle {
                    text: Some(TextStyle {
                        weight: Some(font::Weight::Semibold),
                        ..Default::default()
                    }),
                    fill: Some(neutral_50.into()),
                    borders: Some(BorderStyle {
                        sides: Sides::left(),
                        color: Some(neutral_200),
                        width: Some(1.0),
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::column_labels().columns(["fy_2026"]),
                CellStyle {
                    text: Some(TextStyle {
                        weight: Some(font::Weight::Bold),
                        color: Some(neutral_900),
                        ..Default::default()
                    }),
                    fill: Some(neutral_100.into()),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::summary(),
                CellStyle {
                    text: Some(TextStyle {
                        weight: Some(font::Weight::Semibold),
                        ..Default::default()
                    }),
                    fill: Some(neutral_50.into()),
                    borders: Some(BorderStyle {
                        sides: Sides::top(),
                        color: Some(neutral_200),
                        width: Some(1.0),
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::grand_summary(),
                CellStyle {
                    text: Some(TextStyle {
                        weight: Some(font::Weight::Bold),
                        ..Default::default()
                    }),
                    fill: Some(neutral_100.into()),
                    borders: Some(BorderStyle {
                        sides: Sides::top(),
                        color: Some(neutral_300),
                        width: Some(1.5),
                    }),
                    ..Default::default()
                },
            )
            .tab_style(
                cells::source_notes(),
                CellStyle {
                    text: Some(TextStyle {
                        size: Some(11.0),
                        color: Some(neutral_500),
                        style: Some(font::Style::Italic),
                        align: Some(alignment::Horizontal::Left),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .fmt(
                cells::body()
                    .union(cells::summary())
                    .union(cells::grand_summary()),
                gt::parens_for_negatives(gt::decimal(1)),
            );

        center(scrollable(table)).padding(20).into()
    }
}

const INDENTED_BODY_ROWS: &[usize] = &[1, 2, 4, 5];

fn build_columns() -> Vec<Column> {
    let mut cols = vec![
        Column::text("line_item", "Line Item")
            .width(iced::Length::Fixed(180.0)),
    ];
    for &(id, label) in MONTHS {
        cols.push(Column::numeric(id, label).width(iced::Length::Fixed(64.0)));
    }
    cols.push(
        Column::numeric("fy_2026", "FY 2026").width(iced::Length::Fixed(80.0)),
    );
    cols
}

fn build_rows() -> Vec<Vec<Cell>> {
    [
        ("Revenue", revenue_total_values()),
        ("  Product", product_values()),
        ("  Services", services_values()),
        ("Cost of Goods Sold", cogs_total_values()),
        ("  Materials", materials_values()),
        ("  Labor", labor_values()),
    ]
    .into_iter()
    .map(|(label, monthly)| {
        let fy_total: f64 = monthly.iter().sum();
        std::iter::once(Cell::text(label))
            .chain(monthly.iter().map(|v| Cell::Number(*v)))
            .chain(std::iter::once(Cell::Number(fy_total)))
            .collect()
    })
    .collect()
}

fn build_row_groups() -> Vec<RowGroup> {
    vec![
        RowGroup::new("revenue", vec![0, 1, 2]).label("Revenue"),
        RowGroup::new("cogs", vec![3, 4, 5]).label("Cost of Goods Sold"),
    ]
}

fn build_summary_rows() -> Vec<SummaryRow> {
    vec![
        SummaryRow::group(
            "revenue",
            "Total revenue",
            cells_with_fy(&revenue_total_values()),
        ),
        SummaryRow::group(
            "cogs",
            "Total COGS",
            cells_with_fy(&cogs_total_values()),
        ),
    ]
}

fn build_grand_summary_rows() -> Vec<SummaryRow> {
    let gross: [f64; 12] = std::array::from_fn(|m| {
        revenue_total_values()[m] + cogs_total_values()[m]
    });
    vec![SummaryRow::grand("Gross profit", cells_with_fy(&gross))]
}

fn cells_with_fy(monthly: &[f64; 12]) -> Vec<Cell> {
    let fy: f64 = monthly.iter().sum();
    std::iter::once(Cell::Empty)
        .chain(monthly.iter().map(|v| Cell::Number(*v)))
        .chain(std::iter::once(Cell::Number(fy)))
        .collect()
}

fn revenue_total_values() -> [f64; 12] {
    let p = product_values();
    let s = services_values();
    std::array::from_fn(|m| p[m] + s[m])
}

fn cogs_total_values() -> [f64; 12] {
    let m_ = materials_values();
    let l = labor_values();
    std::array::from_fn(|m| m_[m] + l[m])
}

const MONTHS: &[(&str, &str)] = &[
    ("feb", "Feb"),
    ("mar", "Mar"),
    ("apr", "Apr"),
    ("may", "May"),
    ("jun", "Jun"),
    ("jul", "Jul"),
    ("aug", "Aug"),
    ("sep", "Sep"),
    ("oct", "Oct"),
    ("nov", "Nov"),
    ("dec", "Dec"),
    ("jan", "Jan"),
];

fn product_values() -> [f64; 12] {
    [
        80.0, 88.0, 96.0, 100.0, 108.0, 112.0, 118.0, 122.0, 126.0, 132.0,
        134.0, 140.0,
    ]
}

fn services_values() -> [f64; 12] {
    [
        40.0, 44.0, 49.0, 50.0, 50.0, 50.0, 50.0, 52.0, 54.0, 56.0, 58.0, 60.0,
    ]
}

fn materials_values() -> [f64; 12] {
    [
        -30.0, -33.0, -36.0, -38.0, -40.0, -41.0, -42.0, -44.0, -45.0, -47.0,
        -48.0, -50.0,
    ]
}

fn labor_values() -> [f64; 12] {
    [
        -18.0, -20.0, -22.0, -22.0, -23.0, -24.0, -25.0, -26.0, -27.0, -28.0,
        -29.0, -30.0,
    ]
}
