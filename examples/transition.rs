//! Demonstrates the [`Transition`] widget by cycling a single slot
//! through several different [`Element`] shapes — plain text, a
//! stateful button, a row of items, a column — to verify that swaps
//! between heterogeneous children work, and that the full-bounds
//! slide displacement looks right inside a taller container.
//!
//! Run with: `cargo run --example transition`
//!
//! [`Transition`]: sweeten::widget::transition::Transition
//! [`Element`]: iced::Element

use iced::widget::{button, center, column, container, markdown, row, text};
use iced::{Center, Element, Fill, Theme};

use sweeten::widget::{transition, transition::Direction};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("sweeten • transition")
        .window_size((640.0, 460.0))
        .run()
}

const SLIDE_COUNT: usize = 5;

struct App {
    index: usize,
    direction: Direction,
    slide_clicks: u32,
    markdown: markdown::Content,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            index: 0,
            direction: Direction::Up,
            slide_clicks: 0,
            markdown: markdown::Content::parse(
                "# Three reasons to click\n\
                   - **Why not?**\n\
                   - It's state-perserving\n\
                   - It catches the eye\n",
            ),
            theme: Theme::Oxocarbon,
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    Next,
    Previous,
    DirectionChanged(Direction),
    SlideButtonClicked,
    LinkClicked,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Next => {
                self.index = (self.index + 1) % SLIDE_COUNT;
            }
            Message::Previous => {
                self.index = (self.index + SLIDE_COUNT - 1) % SLIDE_COUNT;
            }
            Message::DirectionChanged(direction) => {
                self.direction = direction;
            }
            Message::SlideButtonClicked => {
                self.slide_clicks += 1;
            }
            Message::LinkClicked => {}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let clicks = self.slide_clicks;

        let banner = transition(self.index, move |&i| match i {
            0 => text("The quick brown fox jumps over the lazy dog.")
                .size(22)
                .into(),
            1 => button(text(format!("Click me! ({clicks} clicks)")))
                .on_press(Message::SlideButtonClicked)
                .padding(12)
                .into(),
            2 => row![
                text("Apples").size(20),
                text("•").size(20),
                text("Oranges").size(20),
                text("•").size(20),
                text("Bananas").size(20),
            ]
            .spacing(15)
            .align_y(Center)
            .into(),
            3 => {
                markdown::view_with(self.markdown.items(), &self.theme, &Viewer)
            }
            _ => text("Amazingly few discotheques provide jukeboxes.")
                .size(22)
                .into(),
        })
        .direction(self.direction)
        .width(Fill)
        .height(Fill);

        let banner_slot = container(banner)
            .padding(10)
            .width(Fill)
            .height(180)
            .style(container::bordered_box);

        let directions = row([
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ]
        .iter()
        .map(|d| direction_btn(d, self.direction == *d)))
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

fn direction_btn<'a>(
    direction: &'a Direction,
    is_active: bool,
) -> Element<'a, Message> {
    button(text(direction.to_string()))
        .on_press(Message::DirectionChanged(*direction))
        .style(if is_active {
            button::primary
        } else {
            button::secondary
        })
        .into()
}

struct Viewer;

impl<'a> markdown::Viewer<'a, Message> for Viewer {
    fn on_link_click(_url: markdown::Uri) -> Message {
        Message::LinkClicked
    }
}
