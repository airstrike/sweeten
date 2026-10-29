//! Demonstrates the enhanced mouse_area widget with position tracking.
//!
//! Run with: `cargo run --example mouse_area`

use iced::widget::{center, column, container, text};
use iced::{Center, Point};
use iced::{Widget, keyboard};

use sweeten::mouse_area;

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .window_size((300, 300))
        .centered()
        .title("sweeten • mouse_area with Point")
        .run()
}

#[derive(Default)]
struct App {
    status: String,
}

#[derive(Clone, Debug)]
enum Message {
    Mouse(&'static str, Point, keyboard::Modifiers),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Mouse(event, p, modifiers) => {
                let shift = if modifiers.shift() { " +shift" } else { "" };
                self.status =
                    format!("{event} at ({:.0}, {:.0}){shift}", p.x, p.y);
            }
        }
    }

    fn view(&self) -> impl Widget<Message> {
        center(
            column![
                mouse_area(
                    center("Hover and click me!").style(container::rounded_box)
                )
                .on_enter(|p, m| Message::Mouse("Entered", p, m))
                .on_exit(|p, m| Message::Mouse("Exited", p, m))
                .on_press(|p, m| Message::Mouse("Left press", p, m))
                .on_release(|p, m| Message::Mouse("Left release", p, m))
                .on_right_press(|p, m| Message::Mouse("Right press", p, m))
                .on_right_release(|p, m| Message::Mouse("Right release", p, m))
                .on_middle_press(|p, m| Message::Mouse("Middle press", p, m))
                .on_middle_release(|p, m| Message::Mouse(
                    "Middle release",
                    p,
                    m
                )),
                text(&self.status).align_x(Center)
            ]
            .spacing(10)
            .align_x(Center),
        )
        .padding(10)
    }
}
