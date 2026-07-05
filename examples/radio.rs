//! Demonstrates sweeten's animated [`radio`].
//!
//! Picking an option fades and scales the dot into the chosen radio
//! while it fades out of the previously-selected one, instead of the
//! dot snapping the moment the selection flips. The selection starts
//! as `None` — the usual state on init — and a "Clear" button resets
//! it so the deselect animation is easy to see.
//!
//! Run with: `cargo run --example radio`
//!
//! [`radio`]: sweeten::widget::radio

use iced::widget::{center, column, container, row, text};
use iced::{Center, Element, Fill, Theme};

use sweeten::widget::{button, radio};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("sweeten • radio")
        .theme(|app: &App| app.theme.clone())
        .window_size((420.0, 360.0))
        .run()
}

struct App {
    selection: Option<Choice>,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selection: None,
            theme: Theme::Oxocarbon,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    A,
    B,
    C,
    All,
}

impl Choice {
    const ALL: [Choice; 4] = [Choice::A, Choice::B, Choice::C, Choice::All];

    fn label(self) -> &'static str {
        match self {
            Choice::A => "A",
            Choice::B => "B",
            Choice::C => "C",
            Choice::All => "All of the above",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Selected(Choice),
    Clear,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Selected(choice) => self.selection = Some(choice),
            Message::Clear => self.selection = None,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let choices = Choice::ALL.into_iter().fold(column![], |col, choice| {
            col.push(
                radio(choice, self.selection)
                    .label(choice.label())
                    .on_toggle(Message::Selected),
            )
        });

        let body = column![
            choices.spacing(14.0),
            row![
                button(text("Clear").size(14.0))
                    .on_press(Message::Clear)
                    .padding([6.0, 14.0])
            ]
            .align_y(Center),
        ]
        .spacing(20.0);

        center(container(body).padding(24.0))
            .width(Fill)
            .height(Fill)
            .into()
    }
}
