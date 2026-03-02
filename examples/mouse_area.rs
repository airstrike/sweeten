//! Demonstrates the enhanced mouse_area widget with position tracking.
//!
//! Run with: `cargo run --example mouse_area`

use iced::widget::{center, column, container, text};
use iced::{Center, Element, Point};

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
    Mouse(&'static str, Point),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Mouse(event, p) => {
                self.status = format!("{event} at ({:.0}, {:.0})", p.x, p.y);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![
                mouse_area(
                    center("Hover and click me!").style(container::rounded_box)
                )
                .on_enter(|p| Message::Mouse("Entered", p))
                .on_exit(|p| Message::Mouse("Exited", p))
                .on_press(|p| Message::Mouse("Left press", p))
                .on_release(|p| Message::Mouse("Left release", p))
                .on_right_press(|p| Message::Mouse("Right press", p))
                .on_right_release(|p| Message::Mouse("Right release", p))
                .on_middle_press(|p| Message::Mouse("Middle press", p))
                .on_middle_release(|p| Message::Mouse("Middle release", p)),
                text(&self.status).align_x(Center)
            ]
            .spacing(10)
            .align_x(Center),
        )
        .padding(10)
        .into()
    }
}
