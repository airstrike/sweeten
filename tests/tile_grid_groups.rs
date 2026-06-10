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
    self, Action, CellHeight, ItemId, State, Width, grid_content, title_bar,
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
        let group_a = state.add_group([0, 0, 6, 4], Width::Shrink, "A");
        let a = state.add_child(group_a, [0, 0, 2, 2], "a").unwrap();
        let group_b = state.add_group([6, 0, 6, 4], Width::Shrink, "B");
        let b = state.add_child(group_b, [0, 0, 2, 2], "b").unwrap();
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
fn size_to_content_renders_without_panic() {
    // Exercises the size-to-content fit (groups resized to children) and
    // group padding — the recursive fitted_engine path.
    let mut app = App::new();
    app.state.fit(true);
    let view: Element<'_, Message> =
        sweeten::tile_grid(&app.state, |_id, d| {
            grid_content(iced::widget::text(*d))
                .title_bar(title_bar(iced::widget::text(*d)))
        })
        .width(Fill)
        .height(Fill)
        .spacing(4)
        .cell_height(CellHeight::Fixed(40.0))
        .group_header(24)
        .group_padding(8)
        .on_action(Message::Grid)
        .into();

    let mut ui = iced_test::Simulator::with_size(
        Default::default(),
        iced::Size::new(W, H),
        view,
    );
    let _ = ui.simulate([Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))]);
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

#[test]
fn wide_tile_does_not_dive_into_group_off_cursor() {
    // Drag tile `a` (in group A) toward group B until its ghost overlaps B's
    // columns, but stop with the cursor still LEFT of group B's box. It must
    // not reparent INTO B: container descent keys off the cursor, not the
    // ghost's far edge (the Image #7 dive).
    //
    // Geometry: 12 cols over 1200px → 100px cells; group B is cols 6–11
    // (x∈[600,1200)). Grab `a` at (30,12); drop at (590,60). The ghost's
    // top-left lands ≈ (560,48) → column 6, so its span (6–7) is inside B's
    // columns, but the cursor (590) is left of B's left edge (600).
    let app = App::new();
    let messages = {
        let mut ui = iced_test::Simulator::with_size(
            Default::default(),
            iced::Size::new(W, H),
            app.view(),
        );
        drag(&mut ui, Point::new(30.0, 12.0), Point::new(590.0, 60.0));
        ui.into_messages().collect::<Vec<_>>()
    };

    let dove_into_b = messages.iter().any(|m| {
        matches!(
            m,
            Message::Grid(Action::Reparent {
                new_parent: Some(p),
                ..
            }) if *p == app.group_b
        )
    });
    assert!(
        !dove_into_b,
        "tile must not reparent into group B while the cursor is outside it; \
         got {messages:?}"
    );
}

// ── Content::controls overlay ───────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Grid(Action) is required by on_action but unread here
enum CtrlMessage {
    Grid(Action),
    Edit(ItemId),
}

struct CtrlApp {
    state: State<&'static str>,
    tile: ItemId,
}

impl CtrlApp {
    fn new() -> Self {
        let mut state: State<&'static str> = State::new(12);
        let group = state.add_group([0, 0, 12, 6], Width::Shrink, "G");
        let tile = state.add_child(group, [0, 0, 3, 3], "t").unwrap();
        Self { state, tile }
    }

    fn view(&self) -> Element<'_, CtrlMessage> {
        let tile = self.tile;
        sweeten::tile_grid(&self.state, move |id, data| {
            let content = grid_content(iced::widget::text(*data))
                .title_bar(title_bar(iced::widget::text(*data)));
            if id == tile {
                content.controls(
                    iced::widget::button(iced::widget::text("E"))
                        .padding(10)
                        .width(40)
                        .on_press(CtrlMessage::Edit(id)),
                )
            } else {
                content
            }
        })
        .width(Fill)
        .height(Fill)
        .spacing(0)
        .cell_height(CellHeight::Fixed(50.0))
        .group_header(0)
        .on_action(CtrlMessage::Grid)
        .into()
    }
}

#[test]
fn controls_overlay_receives_click() {
    // The tile sits at (0,0,300,150); its controls overlay is anchored to
    // the top-right corner (x∈[254,294]), lifted to straddle the top edge
    // (clamped to y=0 for the top row). Clicking it must reach the button.
    let app = CtrlApp::new();
    let messages = {
        let mut ui = iced_test::Simulator::with_size(
            Default::default(),
            iced::Size::new(W, H),
            app.view(),
        );
        // Controls show only on hover: move over the tile first so the
        // overlay appears, then click it.
        ui.point_at(Point::new(150.0, 75.0));
        let _ = ui.simulate([moved(Point::new(150.0, 75.0))]);
        ui.point_at(Point::new(274.0, 12.0));
        let _ = ui.simulate(iced_test::simulator::click());
        ui.into_messages().collect::<Vec<_>>()
    };

    assert!(
        messages
            .iter()
            .any(|m| matches!(m, CtrlMessage::Edit(id) if *id == app.tile)),
        "clicking the controls overlay should emit Edit; got {messages:?}"
    );
}

#[test]
fn controls_overlay_survives_hover_onto_button() {
    // Regression: hovering the *button itself* must not flicker the overlay
    // off. The overlay reports a non-`None` mouse_interaction under the
    // cursor, so iced hands the base widget `Cursor::Unavailable`. If the
    // base then cleared its hover state, the overlay would vanish on the next
    // frame and the click would miss (the user's "50% dead clicks"). Here we
    // move the cursor *onto* the button as a distinct CursorMoved before
    // pressing — pre-fix that drops the overlay and the press reaches nothing.
    let app = CtrlApp::new();
    let messages = {
        let mut ui = iced_test::Simulator::with_size(
            Default::default(),
            iced::Size::new(W, H),
            app.view(),
        );
        // 1. Hover the tile body so the overlay appears.
        ui.point_at(Point::new(150.0, 75.0));
        let _ = ui.simulate([moved(Point::new(150.0, 75.0))]);
        // 2. Move onto the button. With the cursor over the overlay, the
        //    base widget sees `Cursor::Unavailable`; it must keep its hover.
        let on_button = Point::new(274.0, 12.0);
        ui.point_at(on_button);
        let _ = ui.simulate([moved(on_button)]);
        // 3. Now press the button. The overlay must still be there.
        ui.point_at(on_button);
        let _ = ui.simulate([press(), release()]);
        ui.into_messages().collect::<Vec<_>>()
    };

    assert!(
        messages
            .iter()
            .any(|m| matches!(m, CtrlMessage::Edit(id) if *id == app.tile)),
        "the overlay must stay visible while the cursor is on its button, \
         so the press still emits Edit; got {messages:?}"
    );
}

// ── size_to_content child resize keeps cell width constant ──────

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum FitMessage {
    Grid(Action),
}

/// A `size_to_content` group whose two side-by-side children let us measure
/// the inner cell width. `pipeline_w` controls the second child's column span.
fn trends_with_pipeline_width(
    pipeline_w: u16,
) -> iced_test::Simulator<'static, FitMessage> {
    let mut state: State<&'static str> = State::new(12);
    state.fit(true);
    let trends = state.add_group([0, 0, 8, 1], Width::Shrink, "Trends");
    state.add_child(trends, [0, 0, 4, 3], "Bookings").unwrap();
    state
        .add_child(trends, [4, 0, pipeline_w, 3], "Pipeline")
        .unwrap();

    let view: Element<'static, FitMessage> = sweeten::tile_grid(
        // Leak the state so the Simulator can own a 'static view; fine for a
        // one-shot measurement in a test.
        Box::leak(Box::new(state)),
        |_id, data: &&'static str| {
            grid_content(iced::widget::text(*data))
                .title_bar(title_bar(iced::widget::text(*data)))
        },
    )
    .width(Fill)
    .height(Fill)
    .spacing(8)
    .cell_height(CellHeight::Fixed(54.0))
    .group_header(32)
    .group_padding(10)
    .on_action(FitMessage::Grid)
    .into();

    let mut ui = iced_test::Simulator::with_size(
        Default::default(),
        iced::Size::new(1900.0, 1100.0),
        view,
    );
    let _ = ui.simulate([Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))]);
    ui
}

#[test]
fn size_to_content_child_resize_keeps_cell_width() {
    // The "Bookings" tile is always 4 columns wide. The horizontal distance
    // from its left edge to "Pipeline"'s left edge is therefore `4*cell_w +
    // 4*spacing` and depends only on the inner cell width. Freeing a column
    // (Pipeline 4 -> 3) must NOT shrink that cell width: pre-fix the group's
    // body was divided by the authored 8 columns even though it was trimmed
    // to 7, so the cells (and the resized child) shrank — an overshoot that
    // also left a gap to the group border.
    let x_of = |ui: &mut iced_test::Simulator<'_, FitMessage>, label: &str| {
        ui.find(label).unwrap().visible_bounds().unwrap().x
    };

    let mut full = trends_with_pipeline_width(4);
    let span_full = x_of(&mut full, "Pipeline") - x_of(&mut full, "Bookings");

    let mut shrunk = trends_with_pipeline_width(3);
    let span_shrunk =
        x_of(&mut shrunk, "Pipeline") - x_of(&mut shrunk, "Bookings");

    assert!(
        (span_full - span_shrunk).abs() < 2.0,
        "Bookings' 4-column span (cell width) must not change when Pipeline \
         frees a column: full={span_full:.1} shrunk={span_shrunk:.1}"
    );
}

#[test]
fn resizing_one_stacked_tile_must_not_resize_the_other() {
    use iced::widget::container;
    use sweeten::core::widget::Id;

    // Two tiles stacked in a Shrink group, each 2 columns wide. A column is
    // supposed to be a fixed number of pixels, so widening the *top* tile from
    // 2 to 3 must NOT change the *bottom* tile's pixel width.
    //
    // Each tile's body is a `Fill` container with a findable id, so we read the
    // tile's true rendered width (not the text glyph bounds, which never move).
    let measure_bottom = |top_w: u16| -> f32 {
        let mut state: State<&'static str> = State::new(12);
        let g = state.add_group([0, 0, 2, 1], Width::Shrink, "g");
        state.add_child(g, [0, 0, top_w, 1], "top").unwrap();
        state.add_child(g, [0, 1, 2, 1], "bot").unwrap();
        state.fit(true);

        let view: Element<'_, Message> = sweeten::tile_grid(
            Box::leak(Box::new(state)),
            |_id, label: &&'static str| {
                grid_content(
                    container(iced::widget::text(""))
                        .width(Fill)
                        .height(Fill)
                        .id(Id::new(label)),
                )
            },
        )
        .width(Fill)
        .height(Fill)
        .spacing(8)
        .cell_height(CellHeight::Fixed(54.0))
        .group_header(0)
        .group_padding(10)
        .on_action(Message::Grid)
        .into();

        let mut ui = iced_test::Simulator::with_size(
            Default::default(),
            iced::Size::new(1100.0, 760.0),
            view,
        );
        let _ = ui.simulate([Event::Window(
            iced::window::Event::RedrawRequested(std::time::Instant::now()),
        )]);
        ui.find(Id::new("bot")).unwrap().bounds().width
    };

    let with_top_2 = measure_bottom(2);
    let with_top_3 = measure_bottom(3);
    assert!(
        (with_top_2 - with_top_3).abs() < 1.0,
        "widening the top tile (2 -> 3 cols) must not change the bottom \
         tile's pixel width: {with_top_2:.1}px vs {with_top_3:.1}px",
    );
}

#[test]
fn group_keeps_side_gutters_between_frame_and_tiles() {
    use iced::widget::container;
    use sweeten::core::widget::Id;

    // Uniform cells must not cost the group its side gutters: the frame
    // extends a half-gutter beyond the group's columns, so a tile inside a
    // group anchored at the board's left edge sits min(padding, spacing/2)
    // pixels in from x = 0 (the frame's left edge).
    let mut state: State<&'static str> = State::new(12);
    let g = state.add_group([0, 0, 2, 1], Width::Shrink, "g");
    state.add_child(g, [0, 0, 2, 1], "tile").unwrap();
    state.fit(true);

    let view: Element<'_, Message> = sweeten::tile_grid(
        Box::leak(Box::new(state)),
        |_id, label: &&'static str| {
            grid_content(
                container(iced::widget::text(""))
                    .width(Fill)
                    .height(Fill)
                    .id(Id::new(label)),
            )
        },
    )
    .width(Fill)
    .height(Fill)
    .spacing(8)
    .cell_height(CellHeight::Fixed(54.0))
    .group_header(0)
    .group_padding(10)
    .on_action(Message::Grid)
    .into();

    let mut ui = iced_test::Simulator::with_size(
        Default::default(),
        iced::Size::new(1100.0, 760.0),
        view,
    );
    let _ = ui.simulate([Event::Window(iced::window::Event::RedrawRequested(
        std::time::Instant::now(),
    ))]);

    // gutter = min(group_padding, spacing / 2) = min(10, 4) = 4.
    let x = ui.find(Id::new("tile")).unwrap().bounds().x;
    assert!(
        (x - 4.0).abs() < 1.0,
        "a tile in an edge group must sit one gutter in from the frame: \
         expected x ≈ 4.0, got {x:.1}",
    );
}
