//! Demonstrates sweeten's self-animating [`progress_bar`].
//!
//! The bar is driven by two discrete steps — 0 → 50% after 200ms, then
//! → 100% after another 300ms — and the widget itself eases between
//! those steps over 150ms with shadcn's `cubic-bezier(0.4, 0, 0.2, 1)`,
//! so the user sees a smooth fill instead of two jumps.
//!
//! Below the bar sits a fixed-height slot that stays blank during
//! loading and reveals a `Reset` button the moment progress reaches
//! 100%, restarting the cycle.
//!
//! Run with: `cargo run --example progress_bar`
//!
//! [`progress_bar`]: sweeten::widget::progress_bar

use std::time::Duration;

use iced::widget::{button, center, column, container, text};
use iced::{Center, Element, Fill, Task, Theme};

use sweeten::widget::progress_bar;

fn main() -> iced::Result {
    iced::application(Example::new, Example::update, Example::view)
        .title("sweeten • progress_bar")
        .theme(|_: &Example| Theme::Oxocarbon)
        .window_size((360.0, 220.0))
        .run()
}

struct Example {
    progress: f32,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick(f32),
    Reset,
}

impl Example {
    fn new() -> (Self, Task<Message>) {
        (Self { progress: 0.0 }, start())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(value) => {
                self.progress = value;
                // First tick (50%) chains into the second wait; the
                // second tick (100%) is the terminal step.
                if value < 100.0 {
                    Task::future(async {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        Message::Tick(100.0)
                    })
                } else {
                    Task::none()
                }
            }
            Message::Reset => {
                self.progress = 0.0;
                start()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let bar = progress_bar(0.0..=100.0, self.progress)
            .girth(4.0)
            .length(240.0);

        // Fixed-height slot keeps the layout stable while we toggle
        // the Reset button on/off — the column doesn't reflow when
        // the button appears.
        let reset_slot = container(
            (self.progress == 100.0)
                .then_some(button("Reset").on_press(Message::Reset)),
        )
        .height(36.0)
        .center_x(Fill);

        center(
            column![text("Loading…").size(20.0), bar, reset_slot]
                .spacing(16.0)
                .align_x(Center),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }
}

/// Kicks off the 200ms → 50% step. The 300ms → 100% step is chained
/// from inside the [`Message::Tick`] handler.
fn start() -> Task<Message> {
    Task::future(async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        Message::Tick(50.0)
    })
}
