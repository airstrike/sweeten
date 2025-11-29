//! Demonstrates the enhanced pick_list widget with disabled items.
//!
//! This example shows:
//! - `disabled(Fn(&[T]) -> Vec<bool>)` - dynamically disable items
//! - Disabled items are visually distinct and non-selectable
//!
//! Run with: `cargo run --example pick_list`

use iced::widget::{center, column};
use iced::{Center, Element, Fill};

use sweeten::pick_list;

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("sweeten • pick_list with disabled items")
        .window_size((300.0, 200.0))
        .theme(App::theme)
        .run()
}

#[derive(Default)]
struct App {
    selected_language: Option<Language>,
}

#[derive(Clone, Debug)]
enum Message {
    Pick(Language),
}

impl App {
    fn theme(&self) -> iced::Theme {
        iced::Theme::TokyoNightLight
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Pick(option) => {
                self.selected_language = Some(option);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let pick = pick_list(
            &Language::ALL[..],
            self.selected_language,
            Message::Pick,
        )
        .disabled(|languages: &[Language]| {
            languages
                .iter()
                .map(|lang| matches!(lang, Language::Javascript))
                .collect()
        })
        .placeholder("Choose a language...");

        center(
            column![
                "Which is the best programming language?",
                pick,
                self.selected_language
                    .map(|language| match language {
                        Language::Rust => "Correct!",
                        Language::Javascript => "Wrong!",
                        _ => "You must have misclicked... Try again!",
                    })
                    .unwrap_or(""),
            ]
            .width(Fill)
            .align_x(Center)
            .spacing(10),
        )
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Rust,
    Elm,
    Ruby,
    Haskell,
    C,
    Javascript,
    Other,
}

impl Language {
    const ALL: [Language; 7] = [
        Language::C,
        Language::Javascript,
        Language::Elm,
        Language::Ruby,
        Language::Haskell,
        Language::Rust,
        Language::Other,
    ];
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Language::Rust => "Rust",
                Language::Elm => "Elm",
                Language::Ruby => "Ruby",
                Language::Haskell => "Haskell",
                Language::C => "C",
                Language::Javascript => "Javascript",
                Language::Other => "Some other language",
            }
        )
    }
}
