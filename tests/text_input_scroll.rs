//! Regression test for sweeten's stateful horizontal scroll in
//! [`text_input`].
//!
//! Upstream iced recomputes the scroll offset on every frame from the
//! current cursor position — whenever the text overflows the widget,
//! the cursor is pinned to the right edge. Every cursor move therefore
//! *shifts* the visible text: clicking on a character drags that
//! character out from under the pointer, so a follow-up click at the
//! same pixel lands on a different grapheme.
//!
//! Sweeten stores the scroll offset in `State` and only mutates it
//! when the cursor would otherwise leave the visible window. This
//! test reproduces the upstream failure by:
//!
//!   1. Driving everything through a single `Simulator` (fresh
//!      simulators lose focus/cursor/scroll state between interactions).
//!   2. Focusing the input, scrolling the caret all the way right.
//!   3. Clicking the same pixel twice, typing a sentinel after each.
//!      A stable scroll offset means both sentinels land on the same
//!      grapheme, so they sit adjacent in the resulting value.

use std::thread;
use std::time::Duration;

use iced::widget::{Id, container};
use iced::{Element, Fill, Point};

use iced_test::selector;
use iced_test::selector::Target;

use sweeten::widget::text_input;

/// Wall-clock gap longer than iced's 300ms double-click threshold, so
/// two back-to-back simulator clicks register as independent singles
/// instead of collapsing into a select-word double click.
const DOUBLE_CLICK_COOLDOWN: Duration = Duration::from_millis(350);

/// An ASCII payload whose minimum layout width exceeds the 140px
/// `text_input` below, forcing the overflowing codepath.
const LONG_TEXT: &str = "some text that overflows the widget";

#[derive(Debug)]
struct App {
    value: String,
}

#[derive(Debug, Clone)]
enum Message {
    Changed(String),
}

impl App {
    fn new() -> Self {
        Self {
            value: LONG_TEXT.to_string(),
        }
    }

    fn apply(&mut self, messages: Vec<Message>) {
        for Message::Changed(value) in messages {
            self.value = value;
        }
    }

    fn view(&self) -> Element<'_, Message> {
        container(
            text_input("", &self.value)
                .id(Id::new("input"))
                .on_input(Message::Changed)
                .width(140),
        )
        .padding(20)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    }
}

#[test]
fn repeat_click_stays_put() {
    let mut app = App::new();

    let messages = {
        let mut ui = iced_test::simulator(app.view());

        // Locate the input's visible bounds so we can target a
        // specific pixel inside it.
        let target = ui
            .find(selector::id(Id::new("input")))
            .expect("input should exist");
        let Target::TextInput { visible_bounds, .. } = target else {
            panic!("expected TextInput target");
        };
        let bounds = visible_bounds.expect("input should be visible");

        // Focus + scroll caret to the end of the value.
        ui.point_at(Point::new(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        ));
        let _ = ui.simulate(iced_test::simulator::click());
        let _ = ui.tap_key(iced::keyboard::Key::Named(
            iced::keyboard::key::Named::End,
        ));

        let pixel = Point::new(
            bounds.x + bounds.width * 0.5,
            bounds.y + bounds.height * 0.5,
        );

        // First click + sentinel A.
        ui.point_at(pixel);
        let _ = ui.simulate(iced_test::simulator::click());
        let _ = ui.typewrite("A");

        // Wait out the double-click window so the next click is a
        // single click (otherwise `B` would clobber the word
        // containing `A`).
        thread::sleep(DOUBLE_CLICK_COOLDOWN);

        // Second click at the *same* pixel + sentinel B.
        ui.point_at(pixel);
        let _ = ui.simulate(iced_test::simulator::click());
        let _ = ui.typewrite("B");

        ui.into_messages().collect::<Vec<_>>()
    };

    app.apply(messages);

    let a_at = app.value.find('A').expect("A should be inserted");
    let b_at = app.value.find('B').expect("B should be inserted");

    // With a stable scroll offset, B lands on the same grapheme A
    // did — so the two sentinels end up adjacent (B immediately
    // after A, i.e. `b_at == a_at + 1`). Allow a 2-byte slack for
    // any letter-width variation; under the upstream bug the delta
    // explodes into the double digits.
    let delta = b_at as isize - (a_at as isize + 1);
    assert!(
        delta.abs() <= 2,
        "second click drifted from the first: A@{a_at}, B@{b_at} \
         (delta from A+1 = {delta}). value = {:?}",
        app.value,
    );
}
