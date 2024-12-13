use iced::widget::{center, column};
use iced::{Alignment::Center, Element, Fill};

use sweeten::widget::pick_list;

fn main() -> iced::Result {
    iced::application(
        "sweetened iced - PickList example",
        App::update,
        App::view,
    )
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
    Pick(usize, Language),
}

impl App {
    fn theme(&self) -> iced::Theme {
        iced::Theme::TokyoNightLight
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Pick(index, option) => {
                println!("Picked index {index}");
                self.selected_language = Some(option);
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let pick_list = pick_list(
            &Language::ALL[..],
            Some(|languages: &[Language]| {
                languages
                    .iter()
                    .map(|lang| matches!(lang, Language::Javascript))
                    .collect()
            }),
            self.selected_language,
            Message::Pick,
        )
        .placeholder("Choose a language...");

        center(
            column![
                "Which is the best programming language?",
                pick_list,
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
