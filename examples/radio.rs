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
use iced::Task;
use iced::widget::{center, container, text};
use iced::{
    Center, Element, Fill, Subscription, Theme, keyboard, keyboard::key,
};

use sweeten::widget::operation::{focus_next, focus_previous};
use sweeten::widget::{button, checkbox, column, radio, row};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .title("sweeten • radio")
        .theme(|app: &App| app.theme.clone())
        .window_size((520.0, 500.0))
        .settings(iced::Settings {
            default_text_size: 13.0.into(),
            ..Default::default()
        })
        .run()
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Selected(Choice),
    Clear,
    Focused,
    Blurred,
    SetHorizontal(bool),
    FocusNext,
    FocusPrevious,
}

struct App {
    selection: Option<Choice>,
    is_focused: bool,
    horizontal: bool,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            selection: None,
            is_focused: false,
            horizontal: false,
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

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Selected(choice) => self.selection = Some(choice),
            Message::Clear => self.selection = None,
            Message::Focused => self.is_focused = true,
            Message::Blurred => self.is_focused = false,
            Message::SetHorizontal(horizontal) => self.horizontal = horizontal,
            Message::FocusNext => return focus_next().discard(),
            Message::FocusPrevious => return focus_previous().discard(),
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let choices = radio(self.selection, Choice::ALL, |c| c.label())
            .on_select(Message::Selected)
            .on_focus(Message::Focused)
            .on_blur(Message::Blurred)
            .horizontal(self.horizontal)
            .spacing(14);

        let body = column![
            checkbox(self.horizontal)
                .label("Horizontal")
                .on_toggle(Message::SetHorizontal),
            container(choices).height(120),
            text("Press Tab to focus the next widget.\nThe radio group supports keyboard navigation."),
            text(format!("Radio is{}focused.", if self.is_focused { " " } else { " not " })),
            row![
                button(text("Clear"))
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

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Tab),
                modifiers,
                ..
            } => {
                if modifiers.shift() {
                    Some(Message::FocusPrevious)
                } else {
                    Some(Message::FocusNext)
                }
            }
            _ => None,
        })
    }
}
