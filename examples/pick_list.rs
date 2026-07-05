//! Demonstrates the sweetened pick_list widget.
//!
//! This example shows:
//! - `options!` / `group` — titled groups of options in the menu
//! - `None` — a "None" entry that clears the selection
//! - `option(...).disabled()` — disable items inline
//! - `disabled(Fn(&T) -> bool)` — disable items dynamically
//! - `.separator(true)` — rules between groups (spaced by default)
//! - Rich option content — the Rust row carries a ferris svg
//! - Keyboard interaction: Tab focuses the pick list, Enter/Space/arrows
//!   open it, arrow keys, Home/End, and typeahead move the highlighted
//!   option, Enter selects it, and Escape closes the menu
//!
//! Run with: `cargo run --example pick_list`

use std::sync::LazyLock;

use iced::keyboard::{self, key};
use iced::widget::{center, column, row, svg, text};
use iced::{Center, Element, Fill, Subscription, Task, Theme};

use sweeten::pick_list;
use sweeten::widget::operation::focus_next;

static FERRIS: LazyLock<svg::Handle> = LazyLock::new(|| {
    svg::Handle::from_memory(include_bytes!("ferris.svg").as_slice())
});

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title("sweeten • pick_list with groups and disabled items")
        .window_size([400.0, 600.0])
        .settings(iced::Settings {
            default_text_size: 13.0.into(),
            ..Default::default()
        })
        .theme(App::theme)
        .run()
}

#[derive(Default)]
struct App {
    selected: Option<Language>,
}

#[derive(Clone, Debug)]
enum Message {
    Pick(Language),
    Clear,
    Tab,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (App::default(), focus_next().discard())
    }

    fn theme(&self) -> Theme {
        iced::Theme::Oxocarbon
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Pick(option) => {
                self.selected = Some(option);

                Task::none()
            }
            Message::Clear => {
                self.selected = None;

                Task::none()
            }
            Message::Tab => focus_next().discard(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Tab),
                ..
            } => Some(Message::Tab),
            _ => None,
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let pick = pick_list(
            self.selected,
            pick_list::options![
                None,
                pick_list::group(
                    "Imperative",
                    [Language::C, Language::Javascript],
                ),
                pick_list::group(
                    "Functional",
                    [Language::Elm, Language::Haskell, Language::Rust],
                ),
                pick_list::group("Other", [Language::Ruby, Language::Other]),
            ],
            show,
        )
        .typeahead(Language::to_string)
        .on_select(Message::Pick)
        .on_deselect(Message::Clear)
        // .separator(true)
        .disabled(|language| matches!(language, Language::Javascript))
        .placeholder("Choose a language...")
        .radius(8);

        center(
            column![
                "Which is the best programming language?",
                pick,
                self.selected.map(check).unwrap_or(""),
            ]
            .width(Fill)
            .align_x(Center)
            .spacing(10),
        )
        .into()
    }
}

/// Grades a selection.
fn check(language: Language) -> &'static str {
    match language {
        Language::Rust => "Correct!",
        Language::Javascript => "Wrong!",
        _ => "You must have misclicked... Try again!",
    }
}

/// The menu content for a language: ferris accompanies Rust, everyone
/// else is plain text.
#[allow(clippy::trivially_copy_pass_by_ref)] // the view function takes &T
fn show(language: &Language) -> pick_list::Content<'static> {
    match language {
        Language::Rust => pick_list::Content::Element(
            row![
                svg(FERRIS.clone()).width(20).height(13),
                text("Rust").wrapping(text::Wrapping::None)
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
        ),
        other => other.to_string().into(),
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
