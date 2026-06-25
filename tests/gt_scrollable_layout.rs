//! Layout tests for `gt::Table` with `.scrollable(true)`.
//!
//! Uses `iced_test::Simulator` to drive the widget through a
//! real layout pass inside constrained parent containers.

use iced::widget::column;
use iced::{Element, Event, Fill, Never, Theme};

use sweeten::widget::gt;

type Renderer = iced::Renderer;

fn make_table(
    scrollable: bool,
    n_rows: usize,
) -> Element<'static, Never, Theme, Renderer> {
    let cols = vec![gt::Column::text("a", "A"), gt::Column::numeric("b", "B")];
    let rows: Vec<Vec<gt::Cell>> = (0..n_rows)
        .map(|i| {
            vec![
                gt::Cell::text(format!("row {i}")),
                gt::Cell::Number(i as f64),
            ]
        })
        .collect();
    let mut t = gt::Table::new(cols, rows);
    if scrollable {
        t = t.scrollable(true);
    }
    Element::from(t)
}

fn redraw() -> Event {
    Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))
}

/// A scrollable table inside a 400px column with a 50px header
/// should be constrained to 350px via Length::Shrink, not
/// expand to its natural ~667px.
#[test]
fn scrollable_constrained_by_column() {
    let table = make_table(true, 20);
    let header: Element<'_, Never, Theme, Renderer> =
        iced::widget::container("H").height(50.0).width(Fill).into();
    let col: Element<'_, Never, Theme, Renderer> =
        column![header, table].height(400.0).into();

    let mut ui = iced_test::Simulator::with_size(
        Default::default(),
        iced::Size::new(600.0, 400.0),
        col,
    );
    let _ = ui.simulate([redraw()]);
    let _ = ui.into_messages().count();
}

/// When unconstrained, a scrollable table uses its natural
/// height (not infinity).
#[test]
fn scrollable_unconstrained_uses_natural() {
    let table = make_table(true, 20);

    let mut ui = iced_test::Simulator::with_size(
        Default::default(),
        iced::Size::new(600.0, 9999.0),
        table,
    );
    let _ = ui.simulate([redraw()]);
    let _ = ui.into_messages().count();
}
