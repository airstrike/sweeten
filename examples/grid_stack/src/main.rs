//! Demonstrates the GridStack widget for grid-based layouts.
//!
//! This example shows:
//! - Creating a grid with several items
//! - Title bars with controls (pin, close buttons)
//! - Click to focus
//! - Keyboard shortcuts to add/remove items
//! - Item content showing its grid position and size
//! - Styled focused/unfocused items
//! - Moving items by dragging the title bar area
//! - Resizing items by dragging the bottom-right corner/edges
//! - Loading a Google Font (Geist) via fount
//!
//! Run with: `cargo run -p grid_stack`

#[allow(dead_code)]
mod icon;

use iced::keyboard;
use iced::widget::{button, center_y, column, container, row, rule, text};
use iced::{
    Center, Color, Element, Fill, Font, Size, Subscription, Task, Theme, window,
};

use sweeten::widget::grid_stack::{self, GridStack};
use sweeten::widget::grid_stack::{grid_content, title_bar};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window_size((300.0, 200.0))
        .theme(Theme::Light)
        .font(icon::FONT)
        .default_font(Font::with_family("Geist"))
        .title("sweeten - GridStack")
        .run()
}

enum App {
    Loading,
    Loaded(Example),
}

struct Example {
    state: grid_stack::State<Item>,
    items_created: usize,
    focus: Option<grid_stack::ItemId>,
}

#[derive(Clone)]
struct Item {
    id: usize,
    is_pinned: bool,
}

#[derive(Debug, Clone)]
enum Message {
    FontLoaded,
    Clicked(grid_stack::ItemId),
    Moved(grid_stack::MoveEvent),
    Resized(grid_stack::ResizeEvent),
    AddItem,
    TogglePin(grid_stack::ItemId),
    Close(grid_stack::ItemId),
    CloseFocused,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App::Loading,
            Task::future(fount::google::load("Geist", None)).then(|result| {
                match result {
                    Ok(bytes_list) => {
                        Task::batch(bytes_list.into_iter().map(|bytes| {
                            iced::font::load(bytes).map(|_| Message::FontLoaded)
                        }))
                    }
                    Err(_) => Task::done(Message::FontLoaded),
                }
            }),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FontLoaded => {
                if matches!(self, App::Loading) {
                    *self = App::Loaded(Example::new());

                    return window::latest().and_then(|id| {
                        window::resize(id, Size::new(900.0, 700.0))
                    });
                }
                Task::none()
            }
            _ => {
                if let App::Loaded(example) = self {
                    example.update(message);
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            App::Loading => container(text("GridStack").size(32).font(Font {
                weight: iced::font::Weight::Bold,
                ..Font::DEFAULT
            }))
            .center(Fill)
            .into(),
            App::Loaded(example) => example.view(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            App::Loading => Subscription::none(),
            App::Loaded(example) => example.subscription(),
        }
    }
}

impl Example {
    fn new() -> Self {
        let mut state: grid_stack::State<Item> = grid_stack::State::new(12);
        let mut items_created = 0;

        let positions = [
            (0, 0, 4, 2),
            (4, 0, 4, 2),
            (8, 0, 4, 2),
            (0, 2, 6, 3),
            (6, 2, 6, 3),
        ];

        for &(x, y, w, h) in &positions {
            state.add(
                x,
                y,
                w,
                h,
                Item {
                    id: items_created,
                    is_pinned: false,
                },
            );
            items_created += 1;
        }

        Example {
            state,
            items_created,
            focus: None,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Clicked(id) => {
                self.focus = Some(id);
            }
            Message::Moved(event) => {
                self.state.move_item(event.id, event.x, event.y);
            }
            Message::Resized(event) => {
                self.state.resize_item(event.id, event.w, event.h);
            }
            Message::AddItem => {
                let item = Item {
                    id: self.items_created,
                    is_pinned: false,
                };
                if let Some(id) = self.state.add_auto(3, 2, item) {
                    self.focus = Some(id);
                    self.items_created += 1;
                }
            }
            Message::TogglePin(id) => {
                if let Some(item) = self.state.get_mut(id) {
                    item.is_pinned = !item.is_pinned;
                    let pinned = item.is_pinned;
                    self.state.engine_mut().set_item_locked(id, pinned);
                }
            }
            Message::Close(id) => {
                self.state.remove(id);
                if self.focus == Some(id) {
                    self.focus = prev_or_last_item(&self.state, id);
                }
            }
            Message::CloseFocused => {
                if let Some(id) = self.focus
                    && let Some(item) = self.state.get(id)
                    && !item.is_pinned
                {
                    self.state.remove(id);
                    self.focus = prev_or_last_item(&self.state, id);
                }
            }
            Message::FontLoaded => {}
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event
            else {
                return None;
            };

            if !modifiers.command() {
                return None;
            }

            match key.as_ref() {
                keyboard::Key::Character("n") => Some(Message::AddItem),
                keyboard::Key::Character("w") => Some(Message::CloseFocused),
                _ => None,
            }
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let focus = self.focus;
        let total_items = self.state.len();
        let state = &self.state;

        let grid = GridStack::new(state, |id, item| {
            let is_focused = focus == Some(id);

            let title = text!("Item {}", item.id)
                .size(13)
                .color(TITLE_COLOR)
                .font(Font {
                    weight: iced::font::Weight::Bold,
                    ..Font::DEFAULT
                });

            let controls = view_controls(id, total_items, item.is_pinned);

            let mut title_bar = title_bar(title)
                .controls(controls)
                .padding([8, 10])
                .style(if is_focused {
                    style::title_bar_focused
                } else {
                    style::title_bar_active
                });

            if item.is_pinned {
                title_bar = title_bar.always_show_controls();
            }

            let grid_item = state.get_item(id);
            let (gx, gy, gw, gh) = grid_item
                .map(|i| (i.x, i.y, i.w, i.h))
                .unwrap_or((0, 0, 0, 0));

            grid_content(view_content(gx, gy, gw, gh))
                .title_bar(title_bar)
                .style(if is_focused {
                    style::item_focused
                } else {
                    style::item_active
                })
        })
        .width(Fill)
        .height(Fill)
        .spacing(8)
        .on_click(Message::Clicked)
        .on_move(Message::Moved)
        .on_resize(Message::Resized);

        let add_button = button(text("+ Add Item").size(13))
            .on_press(Message::AddItem)
            .style(button::primary)
            .padding([6, 16]);

        let toolbar = container(
            row![
                add_button,
                text!(
                    "{} items  |  {}-column grid  |  Cmd+N / Cmd+W",
                    total_items,
                    state.columns()
                )
                .size(12)
                .color(MUTED_COLOR),
            ]
            .spacing(16)
            .align_y(Center),
        )
        .padding([10, 14]);

        let grid_area =
            container(grid).padding(10).style(style::grid_background);

        column![toolbar, rule::horizontal(1), grid_area].into()
    }
}

const TITLE_COLOR: Color = Color::from_rgb(
    0x33 as f32 / 255.0,
    0x33 as f32 / 255.0,
    0x33 as f32 / 255.0,
);

const MUTED_COLOR: Color = Color::from_rgb(
    0x88 as f32 / 255.0,
    0x88 as f32 / 255.0,
    0x88 as f32 / 255.0,
);

const LABEL_COLOR: Color = Color::from_rgb(
    0x66 as f32 / 255.0,
    0x66 as f32 / 255.0,
    0x66 as f32 / 255.0,
);

/// Focus the item before `id`, or the last item if none precede it.
fn prev_or_last_item<T>(
    state: &grid_stack::State<T>,
    id: grid_stack::ItemId,
) -> Option<grid_stack::ItemId> {
    let ids: Vec<_> = state.iter().map(|(item_id, _)| item_id).collect();
    ids.iter()
        .rev()
        .copied()
        .find(|&item_id| item_id < id)
        .or_else(|| ids.last().copied())
}

fn view_content<'a>(
    gx: u16,
    gy: u16,
    gw: u16,
    gh: u16,
) -> Element<'a, Message> {
    center_y(
        column![
            text!("Position  ({}, {})", gx, gy)
                .center()
                .size(13)
                .color(LABEL_COLOR),
            text!("Size  {} x {}", gw, gh)
                .center()
                .size(13)
                .color(LABEL_COLOR),
        ]
        .spacing(4)
        .width(Fill)
        .align_x(Center),
    )
    .padding(10)
    .into()
}

fn view_controls(
    id: grid_stack::ItemId,
    total_items: usize,
    is_pinned: bool,
) -> Element<'static, Message> {
    let pin_icon = if is_pinned {
        icon::pin_off().size(14)
    } else {
        icon::pin().size(14)
    };
    let pin_btn = button(pin_icon)
        .on_press(Message::TogglePin(id))
        .style(button::text)
        .padding([2, 6]);

    if is_pinned {
        row![pin_btn].spacing(4).into()
    } else {
        let close = button(icon::x().size(14))
            .style(button::text)
            .padding([2, 6])
            .on_press_maybe(if total_items > 1 {
                Some(Message::Close(id))
            } else {
                None
            });
        row![pin_btn, close].spacing(4).into()
    }
}

mod style {
    use iced::widget::container;
    use iced::{Border, Color, Theme};

    const CARD_BORDER: Color = Color::from_rgb(
        0xE0 as f32 / 255.0,
        0xE0 as f32 / 255.0,
        0xE0 as f32 / 255.0,
    );

    const GRID_BG: Color = Color::from_rgb(
        0xF0 as f32 / 255.0,
        0xF1 as f32 / 255.0,
        0xF3 as f32 / 255.0,
    );

    const ACCENT: Color = Color::from_rgb(
        0x58 as f32 / 255.0,
        0x65 as f32 / 255.0,
        0xF2 as f32 / 255.0,
    );

    const RADIUS: f32 = 6.0;

    pub fn title_bar_active(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(Color::BLACK),
            background: Some(Color::WHITE.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(Color::BLACK),
            background: Some(Color::WHITE.into()),
            ..Default::default()
        }
    }

    pub fn item_active(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::WHITE.into()),
            border: Border {
                width: 1.0,
                color: CARD_BORDER,
                radius: RADIUS.into(),
            },
            ..Default::default()
        }
    }

    pub fn item_focused(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::WHITE.into()),
            border: Border {
                width: 2.0,
                color: ACCENT,
                radius: RADIUS.into(),
            },
            ..Default::default()
        }
    }

    pub fn grid_background(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(GRID_BG.into()),
            border: Border {
                width: 1.0,
                color: CARD_BORDER,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}
