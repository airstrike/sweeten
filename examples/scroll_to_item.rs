use std::sync::LazyLock;

use iced::widget::{button, center, column, container, scrollable, sensor, text};
use iced::{widget, Center, Element, Fill, Length, Task};
use iced::advanced::widget::operation::scrollable::scroll_to;
use sweeten::operation::position;
use sweeten::widget::row;

fn main() -> iced::Result {
    iced::application(
        || App::new(),
        App::update,
        App::view,
    )
    .title("sweetened iced • lazy loading example")
    .window_size((800.0, 350.0))
    .centered()
    .run()
}

#[derive(Default)]
struct App;

#[derive(Clone, Debug)]
enum Message {
    JumpTo(usize),
    ItemVisible(usize),
}

const LANGUAGES: &[&str] = &[
    "C#",
    "C++",
    "Clojure",
    "Dart",
    "Elixir",
    "Erlang",
    "F#",
    "Go",
    "Haskell",
    "Java",
    "JavaScript",
    "Kotlin",
    "OCaml",
    "PHP",
    "Python",
    "Ruby",
    "Rust",
    "Scala",
    "Swift",
    "TypeScript",
];

const SCROLLABLE: LazyLock<widget::Id> =
    LazyLock::new(|| widget::Id::new("my_scrollable"));

const POSITION_TRACKER: LazyLock<position::Id> =
    LazyLock::new(|| position::Id::new("languages_row"));

impl App {
    fn new() -> Self {
        Self::default()
    }
    
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::JumpTo(idx) => {
                let _ : Task<Message> = position::find_position(POSITION_TRACKER.clone(), idx).and_then(
                    move |bounds| {
                        println!("Jump to {} at {:?}", idx, bounds.x);
                        scroll_to::<Message>(
                            SCROLLABLE.clone(),
                            scrollable::AbsoluteOffset {
                                x: Some(bounds.x),
                                y: Some(0.0),
                            },
                        );
                        Task::none()
                    },
                );
                Task::none()
            }
            Message::ItemVisible(idx) => {
                println!("Item {} is now visible", idx);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let nav_buttons = column![
            text("Click on a # to jump to that item")
                .width(Fill)
                .center(),
            scrollable(
                row((0..LANGUAGES.len()).map(|idx| {
                    button(text(idx).center())
                        .on_press(Message::JumpTo(idx))
                        .width(Length::Fixed(40.0))
                        .into()
                }))
                .spacing(5.0)
                .padding(10),
            )
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default().width(0).scroller_width(0),
            ))
            .width(Fill)
        ]
        .width(Fill);

        let languages_list = scrollable(
            row(LANGUAGES.iter().enumerate().map(|(idx, &lang)| {
                let item =
                    container(text(lang).center().width(Fill).height(Fill))
                        .width(200.0)
                        .height(200.0)
                        .style(container::rounded_box);

                sensor(container(item).align_y(Center))
                    .on_show(move |_| Message::ItemVisible(idx))
                    .anticipate(100.0)
                    .into()
            }))
            .id(POSITION_TRACKER.clone())
            .spacing(10)
            .align_y(Center),
        )
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ))
        .id(SCROLLABLE.clone())
        .spacing(10)
        .width(Fill);

        center(
            column![nav_buttons, languages_list,]
                .width(Fill)
                .height(Fill)
                .align_x(Center)
                .spacing(20),
        )
        .padding(20)
        .into()
    }
}
