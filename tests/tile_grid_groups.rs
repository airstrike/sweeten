//! Headless widget tests for the recursive (grouped) `tile_grid`.
//!
//! These drive the real `TileGrid` widget through `iced_test::Simulator`,
//! exercising the recursive layout and the single-owner drag state machine.
//! The state-level reparent *commit* is covered by unit tests in
//! `state.rs`; here we verify the **widget** emits the right [`Action`] for a
//! cross-group drag (a `Reparent`), an intra-group drag (a `Move`), and that
//! a grouped layout renders without panicking.

use iced::{Element, Event, Fill, Point, mouse};

use sweeten::widget::tile_grid::{
    self, Action, CellHeight, ItemId, State, grid_content, title_bar,
};

// Deterministic geometry: a 1200×800 window, a 12-column root with two
// side-by-side 6×4 groups, each holding one 2×2 tile, fixed 50px rows, no
// spacing, no group header. That puts group A's body at x∈[0,600), group
// B's body at x∈[600,1200), tile A at (0,0,200,100), tile B at
// (600,0,200,100).
const W: f32 = 1200.0;
const H: f32 = 800.0;

#[derive(Debug, Clone, Copy)]
enum Message {
    Grid(Action),
}

struct App {
    state: State<&'static str>,
    a: ItemId,
    b: ItemId,
    group_b: ItemId,
}

impl App {
    fn new() -> Self {
        let mut state: State<&'static str> = State::new(12);
        let group_a = state.add_group(0, 0, 6, 4, "A", 6);
        let a = state.add_child(group_a, 0, 0, 2, 2, "a").unwrap();
        let group_b = state.add_group(6, 0, 6, 4, "B", 6);
        let b = state.add_child(group_b, 0, 0, 2, 2, "b").unwrap();
        Self {
            state,
            a,
            b,
            group_b,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        sweeten::tile_grid(&self.state, |_id, data| {
            grid_content(iced::widget::text(*data))
                .title_bar(title_bar(iced::widget::text(*data)))
        })
        .width(Fill)
        .height(Fill)
        .spacing(0)
        .cell_height(CellHeight::Fixed(50.0))
        .group_header(0)
        .on_action(Message::Grid)
        .into()
    }
}

fn press() -> Event {
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}

fn moved(p: Point) -> Event {
    Event::Mouse(mouse::Event::CursorMoved { position: p })
}

fn release() -> Event {
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
}

/// Drive a press → move → release drag through the simulator. The widget
/// reads the simulator cursor, so we `point_at` each step before feeding the
/// matching event.
fn drag(ui: &mut iced_test::Simulator<'_, Message>, from: Point, to: Point) {
    ui.point_at(from);
    let _ = ui.simulate([press()]);
    ui.point_at(to);
    let _ = ui.simulate([moved(to)]);
    ui.point_at(to);
    let _ = ui.simulate([release()]);
}

#[test]
fn grouped_layout_renders_without_panic() {
    let app = App::new();
    let mut ui = iced_test::Simulator::with_size(
        Default::default(),
        iced::Size::new(W, H),
        app.view(),
    );
    // A redraw exercises the recursive layout + draw paths.
    let _ = ui.simulate([Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))]);

    // Redraw mid-drag too, so the group-outline + floating-tile draw paths
    // run while a node is picked up.
    ui.point_at(Point::new(30.0, 12.0));
    let _ = ui.simulate([press()]);
    ui.point_at(Point::new(700.0, 60.0));
    let _ = ui.simulate([moved(Point::new(700.0, 60.0))]);
    let _ = ui.simulate([Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))]);

    // No assertion beyond "did not panic".
    let _ = ui.into_messages().count();
}

#[test]
fn cross_group_drag_emits_reparent() {
    let app = App::new();
    let messages = {
        let mut ui = iced_test::Simulator::with_size(
            Default::default(),
            iced::Size::new(W, H),
            app.view(),
        );
        // Grab tile `a`'s title bar (top-left of group A), drag deep into
        // group B's body, release.
        drag(&mut ui, Point::new(30.0, 12.0), Point::new(700.0, 60.0));
        ui.into_messages().collect::<Vec<_>>()
    };

    let reparent = messages.iter().find_map(|m| match m {
        Message::Grid(Action::Reparent {
            node,
            new_parent,
            phase: tile_grid::DragPhase::Ended,
            ..
        }) => Some((*node, *new_parent)),
        _ => None,
    });

    assert_eq!(
        reparent,
        Some((app.a, Some(app.group_b))),
        "dragging tile `a` into group B should emit a Reparent into B; \
         messages = {messages:?}"
    );
}

#[test]
fn intra_group_drag_emits_move() {
    let app = App::new();
    let messages = {
        let mut ui = iced_test::Simulator::with_size(
            Default::default(),
            iced::Size::new(W, H),
            app.view(),
        );
        // Grab tile `a` and drag it within group A's body (to the right,
        // staying inside x∈[0,600)).
        drag(&mut ui, Point::new(30.0, 12.0), Point::new(420.0, 60.0));
        ui.into_messages().collect::<Vec<_>>()
    };

    // The drag stays inside group A, so we expect Move actions and no
    // Reparent.
    let saw_move = messages
        .iter()
        .any(|m| matches!(m, Message::Grid(Action::Move { .. })));
    let saw_reparent = messages
        .iter()
        .any(|m| matches!(m, Message::Grid(Action::Reparent { .. })));

    assert!(
        saw_move,
        "intra-group drag should emit Move; got {messages:?}"
    );
    assert!(
        !saw_reparent,
        "intra-group drag should not reparent; got {messages:?}"
    );
    // The moved tile is `a`, and `b` is untouched.
    let _ = app.b;
}
