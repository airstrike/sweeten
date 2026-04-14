//! Demonstrates sweeten's table widget with optional column headers.
//!
//! The "Show header" checkbox toggles every column's header between
//! `Some(..)` and `None`. When all columns are headerless, the header
//! row (and the separator beneath it) is skipped entirely.
//!
//! Run with: `cargo run --example table`

use iced::font;
use iced::time::{Duration, hours, minutes};
use iced::widget::{
    center_x, center_y, checkbox, column, container, row, scrollable, slider,
    text, tooltip,
};
use iced::{Center, Element, Fill, Font, Right, Theme};

use sweeten::widget::table;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(Theme::CatppuccinMocha)
        .title("sweeten • table with optional header")
        .run()
}

struct App {
    events: Vec<Event>,
    padding: (f32, f32),
    separator: (f32, f32),
    border: f32,
    show_header: bool,
    sticky_header: bool,
}

#[derive(Debug, Clone)]
enum Message {
    PaddingChanged(f32, f32),
    SeparatorChanged(f32, f32),
    BorderChanged(f32),
    ShowHeaderToggled(bool),
    StickyHeaderToggled(bool),
}

impl App {
    fn new() -> Self {
        Self {
            events: Event::list(),
            padding: (10.0, 5.0),
            separator: (1.0, 1.0),
            border: 0.0,
            show_header: true,
            sticky_header: true,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PaddingChanged(x, y) => self.padding = (x, y),
            Message::SeparatorChanged(x, y) => self.separator = (x, y),
            Message::BorderChanged(width) => self.border = width,
            Message::ShowHeaderToggled(show) => self.show_header = show,
            Message::StickyHeaderToggled(sticky) => {
                self.sticky_header = sticky;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let show_header = self.show_header;
        let header =
            |label: &'static str| show_header.then(|| bold_header(label));

        let table = {
            let columns = [
                table::column(header("Name"), |event: &Event| {
                    text(&event.name)
                }),
                table::column(header("Time"), |event: &Event| {
                    let minutes = event.duration.as_secs() / 60;

                    text!("{minutes} min").style(if minutes > 90 {
                        text::warning
                    } else {
                        text::default
                    })
                })
                .align_x(Right)
                .align_y(Center),
                table::column(header("Price"), |event: &Event| {
                    if event.price > 0.0 {
                        text!("${:.2}", event.price).style(
                            if event.price > 100.0 {
                                text::warning
                            } else {
                                text::default
                            },
                        )
                    } else {
                        text("Free").style(text::success).width(Fill).center()
                    }
                })
                .align_x(Right)
                .align_y(Center),
                table::column(header("Rating"), |event: &Event| {
                    text!("{:.2}", event.rating).style(if event.rating > 4.7 {
                        text::success
                    } else if event.rating < 2.0 {
                        text::danger
                    } else {
                        text::default
                    })
                })
                .align_x(Right)
                .align_y(Center),
            ];

            table(columns, &self.events)
                .padding_x(self.padding.0)
                .padding_y(self.padding.1)
                .separator_x(self.separator.0)
                .separator_y(self.separator.1)
                .border(self.border)
                .sticky_header(self.sticky_header)
        };

        let controls = {
            let labeled_slider =
                |label,
                 range: std::ops::RangeInclusive<f32>,
                 (x, y),
                 on_change: fn(f32, f32) -> Message| {
                    row![
                        text(label).font(Font::MONOSPACE).size(14).width(100),
                        tooltip(
                            slider(range.clone(), x, move |x| on_change(x, y)),
                            text!("{x:.0}px").font(Font::MONOSPACE).size(10),
                            tooltip::Position::Left
                        ),
                        tooltip(
                            slider(range, y, move |y| on_change(x, y)),
                            text!("{y:.0}px").font(Font::MONOSPACE).size(10),
                            tooltip::Position::Right
                        ),
                    ]
                    .spacing(10)
                    .align_y(Center)
                };

            let border_slider = row![
                text("Border").font(Font::MONOSPACE).size(14).width(100),
                tooltip(
                    slider(0.0..=10.0, self.border, Message::BorderChanged),
                    text!("{:.0}px", self.border)
                        .font(Font::MONOSPACE)
                        .size(10),
                    tooltip::Position::Right,
                ),
            ]
            .spacing(10)
            .align_y(Center);

            column![
                checkbox(self.show_header)
                    .label("Show header")
                    .on_toggle(Message::ShowHeaderToggled),
                checkbox(self.sticky_header)
                    .label("Sticky header")
                    .on_toggle(Message::StickyHeaderToggled),
                labeled_slider(
                    "Padding",
                    0.0..=30.0,
                    self.padding,
                    Message::PaddingChanged
                ),
                labeled_slider(
                    "Separator",
                    0.0..=5.0,
                    self.separator,
                    Message::SeparatorChanged
                ),
                border_slider,
            ]
            .spacing(10)
            .width(400)
        };

        column![
            center_y(scrollable(center_x(table)).spacing(10)).padding(10),
            center_x(controls).padding(10).style(container::dark)
        ]
        .into()
    }
}

fn bold_header<Message: 'static>(
    label: &'static str,
) -> Element<'static, Message> {
    text(label)
        .font(Font {
            weight: font::Weight::Bold,
            ..Font::DEFAULT
        })
        .into()
}

struct Event {
    name: String,
    duration: Duration,
    price: f32,
    rating: f32,
}

impl Event {
    fn list() -> Vec<Self> {
        vec![
            Event {
                name: "Get lost in a hacker bookstore".to_owned(),
                duration: hours(2),
                price: 0.0,
                rating: 4.9,
            },
            Event {
                name: "Buy vintage synth at Noisebridge flea market".to_owned(),
                duration: hours(1),
                price: 150.0,
                rating: 4.8,
            },
            Event {
                name: "Eat a questionable hot dog at 2AM".to_owned(),
                duration: minutes(20),
                price: 5.0,
                rating: 1.7,
            },
            Event {
                name: "Ride the MUNI for the story".to_owned(),
                duration: minutes(60),
                price: 3.0,
                rating: 4.1,
            },
            Event {
                name: "Scream into the void from Twin Peaks".to_owned(),
                duration: minutes(40),
                price: 0.0,
                rating: 4.9,
            },
            Event {
                name: "Buy overpriced coffee and feel things".to_owned(),
                duration: minutes(25),
                price: 6.5,
                rating: 4.5,
            },
            Event {
                name: "Attend an underground robot poetry slam".to_owned(),
                duration: hours(1),
                price: 12.0,
                rating: 4.8,
            },
            Event {
                name: "Browse cursed tech at a retro computer fair".to_owned(),
                duration: hours(2),
                price: 10.0,
                rating: 4.7,
            },
            Event {
                name: "Try to order at a secret ramen place with no sign"
                    .to_owned(),
                duration: minutes(50),
                price: 14.0,
                rating: 4.6,
            },
            Event {
                name: "Join a spontaneous rooftop drone rave".to_owned(),
                duration: hours(3),
                price: 0.0,
                rating: 4.9,
            },
            Event {
                name: "Sketch a stranger at Dolores Park".to_owned(),
                duration: minutes(45),
                price: 0.0,
                rating: 4.4,
            },
            Event {
                name: "Visit the Museum of Obsolete APIs".to_owned(),
                duration: hours(1),
                price: 9.99,
                rating: 4.2,
            },
            Event {
                name: "Chase the last working payphone".to_owned(),
                duration: minutes(35),
                price: 0.25,
                rating: 4.0,
            },
            Event {
                name: "Trade zines with a punk on BART".to_owned(),
                duration: minutes(30),
                price: 3.5,
                rating: 4.7,
            },
            Event {
                name: "Get a tattoo of the Git logo".to_owned(),
                duration: hours(1),
                price: 200.0,
                rating: 4.6,
            },
        ]
    }
}
