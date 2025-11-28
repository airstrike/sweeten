//! Demonstrates the enhanced mouse_area widget with click position tracking.
//!
//! This example shows how all mouse event handlers receive the cursor position
//! as a [`Point`] in local coordinates.
//!
//! Run with: `cargo run --example mouse_area`

use iced::widget::{center, column, container, row, text};
use iced::{Center, Element, Length, Point, color};

use sweeten::mouse_area;

fn main() -> iced::Result {
    iced::run(App::update, App::view)
}

struct App {
    last_click: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            last_click: String::from("Click a block!"),
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    ClickWithPoint(Point),
    SimpleClick,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::ClickWithPoint(point) => {
                self.last_click =
                    format!("Clicked at ({:.0}, {:.0})", point.x, point.y);
            }
            Message::SimpleClick => {
                self.last_click = String::from("Simple click");
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![
                row![
                    mouse_area(block(
                        "Click me and I'll tell you where!",
                        0x813060
                    ))
                    .on_press(Message::ClickWithPoint),
                    mouse_area(block(
                        "Click me and I won't say a word...",
                        0x008189
                    ))
                    .on_press(|_| Message::SimpleClick),
                ]
                .spacing(10),
                text(&self.last_click).align_x(Center)
            ]
            .width(Length::Shrink)
            .align_x(Center)
            .spacing(10),
        )
        .into()
    }
}

fn block<'a>(label: &'a str, hex: u32) -> Element<'a, Message> {
    container(label)
        .align_y(Center)
        .align_x(Center)
        .width(Length::Fixed(300.0))
        .height(Length::Fixed(200.0))
        .style(move |_| container::background(color!(hex)))
        .into()
}
