//! Demonstrates the [`FitText`] widget: a headline whose font size scales
//! up and down to fit the bounds it is laid out into, while staying within
//! a configurable `[min_size, max_size]` range.
//!
//! Type into the text field and resize the window to watch the headline
//! scale to fit.
//!
//! Run with: `cargo run --example fit_text`
//!
//! [`FitText`]: sweeten::widget::fit_text::FitText

use iced::widget::{center, column, container, row, slider, text};
use iced::{Center, Element, Fill, Shrink};

use sweeten::widget::{fit_text, text_input};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("sweeten • fit_text")
        .theme(iced::Theme::Ferra)
        .window_size((720.0, 420.0))
        .run()
}

struct App {
    headline: String,
    max_size: f32,
    min_size: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            headline: "Headline that fits".to_string(),
            max_size: 120.0,
            min_size: 16.0,
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    EditHeadline(String),
    SetMax(f32),
    SetMin(f32),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::EditHeadline(s) => self.headline = s,
            Message::SetMax(size) => {
                self.max_size = size;
                if self.min_size > self.max_size {
                    self.min_size = self.max_size;
                }
            }
            Message::SetMin(size) => {
                self.min_size = size;
                if self.max_size < self.min_size {
                    self.max_size = self.min_size;
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let headline = fit_text(&self.headline)
            .max_size(self.max_size)
            .min_size(self.min_size)
            .width(Fill)
            .height(Fill)
            .center();

        let stage = container(headline)
            .padding(12)
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(container::bordered_box);

        let input = text_input("Type a headline...", &self.headline)
            .on_input(Message::EditHeadline)
            .size(18);

        let max_row = row![
            text("max").width(40.0),
            slider(4.0..=240.0, self.max_size, Message::SetMax)
                .step(1.0)
                .width(Fill),
            text(format!("{:>3.0}px", self.max_size)).width(56.0),
        ]
        .spacing(12)
        .align_y(Center);

        let min_row = row![
            text("min").width(40.0),
            slider(4.0..=240.0, self.min_size, Message::SetMin)
                .step(1.0)
                .width(Fill),
            text(format!("{:>3.0}px", self.min_size)).width(56.0),
        ]
        .spacing(12)
        .align_y(Center);

        let controls = column![input, max_row, min_row]
            .spacing(12)
            .width(Fill)
            .height(Shrink);

        center(
            column![stage, controls]
                .spacing(16)
                .width(Fill)
                .height(Fill),
        )
        .padding(20)
        .into()
    }
}
