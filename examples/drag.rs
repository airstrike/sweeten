use iced::Length::Fill;
use iced::widget::{button, container, text};
use iced::{Center, Element, Task};

use sweeten::widget::drag::DragEvent;
use sweeten::widget::{column, row};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("sweeten • drag and drop")
        .window_size((400, 400))
        .run()
}

#[derive(Default)]
struct App {
    elements: Vec<&'static str>,
    mode: Mode,
}

#[derive(Debug, Clone, Default, PartialEq)]
enum Mode {
    Row,
    #[default]
    Column,
}

#[derive(Debug, Clone)]
enum Message {
    Reorder(DragEvent),
    SwitchMode(Mode),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                elements: vec![
                    "Apple",
                    "Banana",
                    "Cherry",
                    "Date",
                    "Elderberry",
                ],
                ..Default::default()
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SwitchMode(mode) => {
                self.mode = mode;
            }
            Message::Reorder(event) => match event {
                DragEvent::Picked { .. } => {
                    // Optionally handle pick event
                }
                DragEvent::Dropped {
                    index,
                    target_index,
                } => {
                    // Update self.elements based on index and target_index
                    let item = self.elements.remove(index);
                    self.elements.insert(target_index, item);
                }
                DragEvent::Canceled { .. } => {
                    // Optionally handle cancel event
                }
            },
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let items = self.elements.iter().copied().map(pickme);
        let drag: Element<'_, Message> = match self.mode {
            Mode::Column => column(items)
                .spacing(5)
                .deadband_zone(0.0)
                .on_drag(Message::Reorder)
                .align_x(Center)
                .into(),
            Mode::Row => row(items)
                .spacing(5)
                .on_drag(Message::Reorder)
                .style(|_| row::Style {
                    scale: 1.5,
                    moved_item_overlay: iced::Color::BLACK.scale_alpha(0.75),
                    ghost_background: iced::color![170, 0, 0]
                        .scale_alpha(0.25)
                        .into(),
                    ghost_border: iced::Border {
                        color: iced::Color::TRANSPARENT,
                        width: 0.0,
                        radius: 5.0.into(),
                    },
                })
                .align_y(Center)
                .into(),
        };

        container(
            column![
                row![
                    text("Drag items around!").width(Fill),
                    button(text("ROW").size(12))
                        .on_press(Message::SwitchMode(Mode::Row))
                        .style(if self.mode == Mode::Row {
                            button::primary
                        } else {
                            button::subtle
                        }),
                    button(text("COLUMN").size(12))
                        .on_press(Message::SwitchMode(Mode::Column))
                        .style(if self.mode == Mode::Column {
                            button::primary
                        } else {
                            button::subtle
                        }),
                ]
                .spacing(5)
                .align_y(Center),
                container(drag)
                    .padding(20)
                    .center(Fill)
                    .style(container::bordered_box)
            ]
            .align_x(Center)
            .spacing(5),
        )
        .padding(20)
        .center(Fill)
        .into()
    }
}

fn pickme(label: &str) -> Element<'_, Message> {
    container(text(label))
        .style(container::rounded_box)
        .padding(5)
        .into()
}
