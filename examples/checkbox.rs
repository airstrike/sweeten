//! Demonstrates sweeten's animated [`checkbox`].
//!
//! Each click fades and scales the checkmark in or out while the
//! background and border colors interpolate between the off- and
//! on-state styles, instead of snapping the moment `is_checked`
//! flips. A "Toggle all" button drives several checkboxes at once
//! so the animations are easy to see in concert.
//!
//! Run with: `cargo run --example checkbox`
//!
//! [`checkbox`]: sweeten::widget::checkbox

use iced::widget::{center, column, container, row, text};
use iced::{Center, Element, Fill, Theme};

use sweeten::widget::{button, checkbox};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("sweeten • checkbox")
        .theme(|app: &App| app.theme.clone())
        .window_size((420.0, 360.0))
        .run()
}

struct App {
    primary: bool,
    secondary: bool,
    success: bool,
    danger: bool,
    text: bool,
    disabled: bool,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            primary: true,
            secondary: false,
            success: false,
            danger: true,
            text: true,
            disabled: true,
            theme: Theme::Oxocarbon,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Primary(bool),
    Secondary(bool),
    Success(bool),
    Danger(bool),
    Text(bool),
    ToggleAll,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Primary(v) => self.primary = v,
            Message::Secondary(v) => self.secondary = v,
            Message::Success(v) => self.success = v,
            Message::Danger(v) => self.danger = v,
            Message::Text(v) => self.text = v,
            Message::ToggleAll => {
                let any_off = !(self.primary
                    && self.secondary
                    && self.success
                    && self.danger
                    && self.text);
                self.primary = any_off;
                self.secondary = any_off;
                self.success = any_off;
                self.danger = any_off;
                self.text = any_off;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let body = column![
            checkbox(self.primary)
                .label("Primary")
                .on_toggle(Message::Primary)
                .style(sweeten::widget::checkbox::primary),
            checkbox(self.secondary)
                .label("Secondary")
                .on_toggle(Message::Secondary)
                .style(sweeten::widget::checkbox::secondary),
            checkbox(self.success)
                .label("Success")
                .on_toggle(Message::Success)
                .style(sweeten::widget::checkbox::success),
            checkbox(self.danger)
                .label("Danger")
                .on_toggle(Message::Danger)
                .style(sweeten::widget::checkbox::danger),
            checkbox(self.text)
                .label("Text")
                .on_toggle(Message::Text)
                .style(sweeten::widget::checkbox::text),
            checkbox(self.disabled).label("Disabled"),
            row![
                button(text("Toggle all").size(14.0))
                    .on_press(Message::ToggleAll)
                    .padding([6.0, 14.0])
            ]
            .align_y(Center),
        ]
        .spacing(14.0);

        center(container(body).padding(24.0))
            .width(Fill)
            .height(Fill)
            .into()
    }
}
