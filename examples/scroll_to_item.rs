//! Demonstrates scroll-to-item functionality using position tracking.
//!
//! This example shows:
//! - Setting a [`position::Id`] on a [`Column`] to enable position tracking
//! - Using [`position::find_position`] to query a child's layout bounds
//! - Scrolling to a specific item with [`scroll_to`]
//!
//! Run with: `cargo run --example scroll_to_item`

use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Id, container, scrollable, text};
use iced::{Element, Fill, Task};

use sweeten::widget::operation::position;
use sweeten::widget::{button, column, row};

const ITEM_COUNT: usize = 100;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("sweeten - scroll to item")
        .window_size((500.0, 600.0))
        .run()
}

struct App {
    column_id: position::Id,
    scrollable_id: Id,
}

#[derive(Debug, Clone)]
enum Message {
    ScrollTo(usize),
    ScrollResult(Option<iced::Rectangle>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                column_id: position::Id::unique(),
                scrollable_id: Id::unique(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScrollTo(index) => {
                return position::find_position(self.column_id.clone(), index)
                    .map(Message::ScrollResult);
            }
            Message::ScrollResult(Some(bounds)) => {
                return iced::widget::operation::scroll_to(
                    self.scrollable_id.clone(),
                    AbsoluteOffset {
                        x: None,
                        y: Some(bounds.y),
                    },
                );
            }
            Message::ScrollResult(None) => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let items = (0..ITEM_COUNT).map(|i| {
            container(text(format!("Item {i}")))
                .style(container::rounded_box)
                .padding(10)
                .width(Fill)
                .into()
        });

        let buttons: Vec<Element<'_, Message>> = [0, 10, 25, 50, 75, 99]
            .iter()
            .map(|&i| {
                button(text(format!("Item {i}")).size(12))
                    .on_press(Message::ScrollTo(i))
                    .into()
            })
            .collect();

        let sidebar = column(buttons).spacing(5).width(100);

        let content = scrollable(
            column(items)
                .id(self.column_id.clone())
                .spacing(5)
                .width(Fill),
        )
        .id(self.scrollable_id.clone())
        .spacing(10)
        .height(Fill)
        .width(Fill);

        row![sidebar, content].spacing(10).padding(20).into()
    }
}
