//! Demonstrates the [`Transition`] widget — a banner-style slot whose
//! contents animate in from a configurable [`Direction`] when the value
//! changes.
//!
//! This example shows:
//! - Cycling a list of phrases through a single transition slot
//! - Picking the slide direction at runtime
//! - Reordering preserves the prior content while the new one slides in
//!
//! Run with: `cargo run --example transition`
//!
//! [`Transition`]: sweeten::widget::transition::Transition
//! [`Direction`]: sweeten::widget::transition::Direction

use iced::widget::{button, center, column, container, row, text};
use iced::{Center, Element, Fill};

use sweeten::widget::{transition, transition::Direction};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("sweeten • transition")
        .window_size((480.0, 280.0))
        .run()
}

const PHRASES: &[&str] = &[
    "The quick brown fox jumps over the lazy dog.",
    "Amazingly few discotheques provide jukeboxes.",
    "Sphinx of black quartz, judge my vow.",
    "Pack my box with five dozen liquor jugs.",
    "How vexingly quick daft zebras jump!",
];

struct App {
    index: usize,
    direction: Direction,
}

impl Default for App {
    fn default() -> Self {
        Self {
            index: 0,
            direction: Direction::Up,
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    Next,
    Previous,
    DirectionChanged(Direction),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Next => {
                self.index = (self.index + 1) % PHRASES.len();
            }
            Message::Previous => {
                self.index = (self.index + PHRASES.len() - 1) % PHRASES.len();
            }
            Message::DirectionChanged(direction) => {
                self.direction = direction;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let phrase: String = PHRASES[self.index].to_string();

        let banner =
            transition(phrase, |s: &String| text(s.clone()).size(22).into())
                .height(60)
                .direction(self.direction)
                .align_x(Center)
                .align_y(Center)
                .width(Fill);

        let banner_slot = container(banner).style(container::bordered_box);

        let directions = row([
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .iter()
        .map(|d| btn(d, self.direction == *d)))
        .spacing(5);

        let controls = row![
            button("Previous").on_press(Message::Previous),
            button("Next").on_press(Message::Next),
        ]
        .spacing(5)
        .align_y(Center);

        center(
            column![banner_slot, directions, controls]
                .width(Fill)
                .align_x(Center)
                .spacing(20),
        )
        .padding(20)
        .into()
    }
}

fn btn<'a>(direction: &'a Direction, is_active: bool) -> Element<'a, Message> {
    button(text(direction.to_string()))
        .on_press(Message::DirectionChanged(*direction))
        .style(if is_active {
            button::primary
        } else {
            button::secondary
        })
        .into()
}
