use iced::widget::{container, horizontal_space, stack};
use iced::Length::Fill;
use iced::{Color, Element};
use sweeten::widget::local_text_input;

#[derive(Debug, Clone)]
enum Message {
    AdjustColor(String),
}

#[derive(Debug, Clone)]
struct App {
    color: Color,
}

const INITIAL_COLOR: &'static str = "#163832";

impl Default for App {
    fn default() -> Self {
        Self {
            color: Color::parse(INITIAL_COLOR).unwrap(),
        }
    }
}

impl App {
    pub fn view(&self) -> Element<Message> {
        container(
            stack![
                container(horizontal_space()).center(Fill).style(
                    move |_: &iced::Theme| {
                        container::Style::default().background(self.color)
                    }
                ),
                container(
                    local_text_input("Enter color", INITIAL_COLOR)
                        .on_submit(Message::AdjustColor)
                        .on_blur(Message::AdjustColor)
                        .width(300)
                        .padding(10)
                )
                .center(Fill)
            ]
            .width(Fill)
            .height(Fill),
        )
        .center(Fill)
        .into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::AdjustColor(color) => {
                if let Some(new_color) = Color::parse(&color) {
                    self.color = new_color;
                }
            }
        }
    }
}

fn main() -> iced::Result {
    iced::run(
        "sweetened iced - LocalTextInput example",
        App::update,
        App::view,
    )
}
