use iced::widget::{button, container, horizontal_space, row, stack};
use iced::Alignment::Center;
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

const INITIAL_COLOR: &str = "#163832";

impl Default for App {
    fn default() -> Self {
        Self {
            color: Color::parse(INITIAL_COLOR).unwrap(),
        }
    }
}

impl App {
    pub fn color_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.color.r * 255.0) as u8,
            (self.color.g * 255.0) as u8,
            (self.color.b * 255.0) as u8
        )
    }

    pub fn view(&self) -> Element<Message> {
        container(
            stack![
                container(horizontal_space()).center(Fill).style(
                    move |_: &iced::Theme| {
                        container::Style::default().background(self.color)
                    }
                ),
                container(
                    row![
                        local_text_input("Enter color", &self.color_hex())
                            .on_submit(Message::AdjustColor)
                            .on_blur(Message::AdjustColor)
                            .width(300),
                        button("Reset").style(button::secondary).on_press(
                            Message::AdjustColor(INITIAL_COLOR.to_string())
                        )
                    ]
                    .align_y(Center)
                    .spacing(8)
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
