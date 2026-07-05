//! Demonstrates sweeten's focus-managed [`radio`] group.
//!
//! The whole set of buttons is a single tab stop (the WAI-ARIA
//! "radiogroup"): press Tab to move focus between the group and the
//! "Clear" button, then use the arrow keys to rove between options — the
//! selection follows focus — or Space to select the focused one. Picking an
//! option fades and scales the dot into the chosen radio while it fades out
//! of the previously-selected one, instead of the dot snapping the moment
//! the selection flips.
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
    is_focused: bool,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selection: None,
            is_focused: false,
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
    Focused,
    Blurred,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Selected(choice) => self.selection = Some(choice),
            Message::Clear => self.selection = None,
            Message::Focused => self.is_focused = true,
            Message::Blurred => self.is_focused = false,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let choices = radio(self.selection, Choice::ALL, |c| c.label())
            .on_select(Message::Selected)
            .on_focus(Message::Focused)
            .on_blur(Message::Blurred)
            .spacing(14.0);

        let hint = if self.is_focused {
            "Group focused — arrow keys move, Space selects"
        } else {
            "Tab to focus the group"
        };

        let body = column![
            choices,
            text(hint).size(13.0),
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
