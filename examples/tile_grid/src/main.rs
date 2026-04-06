//! Demonstrates the TileGrid widget for grid-based layouts.
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
//! Run with: `cargo run -p tile_grid`

#[allow(dead_code)]
mod icon;

use iced::keyboard;
use iced::widget::{
    button, center_y, column, container, row, rule, scrollable, space, text,
};
use iced::{
    Center, Element, Fill, Font, Shrink, Size, Subscription, Task, window,
};

use sweeten::widget::tile_grid::{self, grid_content, title_bar};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window_size((300.0, 200.0))
        .theme(iced::Theme::GruvboxDark)
        .font(icon::FONT)
        .default_font(Font::with_family("Geist"))
        .title("sweeten - TileGrid")
        .run()
}

enum App {
    Loading,
    Loaded(Example),
}

struct Example {
    state: tile_grid::State<Item>,
    items_created: usize,
    focus: Option<tile_grid::ItemId>,
    locked_all: bool,
}

#[derive(Clone)]
struct Item {
    id: usize,
    is_pinned: bool,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    FontLoaded,
    GridAction(tile_grid::Action),
    AddItem,
    AddTen,
    TogglePin(tile_grid::ItemId),
    Close(tile_grid::ItemId),
    CloseFocused,
    ToggleLockAll,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App::Loading,
            Task::future(fount::google::load_variants(
                "Geist",
                &["400", "700"],
            ))
            .then(|result| match result {
                Ok(bytes_list) => {
                    Task::batch(bytes_list.into_iter().map(|bytes| {
                        iced::font::load(bytes).map(|_| Message::FontLoaded)
                    }))
                }
                Err(e) => {
                    eprintln!("Failed to load font: {e}");
                    Task::done(Message::FontLoaded)
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
            App::Loading => container(text("TileGrid").size(32).font(Font {
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
        let mut state: tile_grid::State<Item> = tile_grid::State::new(12);
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
            locked_all: false,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::GridAction(action) => {
                // Track focus on click
                if action.is_click() {
                    self.focus = Some(action.id());
                }

                // Perform the action (handles batch mode, held items, everything)
                self.state.perform(action, |_, item| item.is_pinned);
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
            Message::AddTen => {
                // Cycle through a few sizes to make the stress layout
                // visually interesting.
                let sizes = [(3, 2), (4, 2), (2, 2), (6, 3), (4, 3)];
                for i in 0..10 {
                    let (w, h) = sizes[i % sizes.len()];
                    let item = Item {
                        id: self.items_created,
                        is_pinned: false,
                    };
                    if let Some(id) = self.state.add_auto(w, h, item) {
                        self.focus = Some(id);
                        self.items_created += 1;
                    }
                }
            }
            Message::TogglePin(id) => {
                if let Some(item) = self.state.get_mut(id) {
                    item.is_pinned = !item.is_pinned;
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
            Message::ToggleLockAll => {
                self.locked_all = !self.locked_all;
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
                keyboard::Key::Character("n") if modifiers.shift() => {
                    Some(Message::AddTen)
                }
                keyboard::Key::Character("n") => Some(Message::AddItem),
                keyboard::Key::Character("N") => Some(Message::AddTen),
                keyboard::Key::Character("w") => Some(Message::CloseFocused),
                _ => None,
            }
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let focus = self.focus;
        let total_items = self.state.len();
        let state = &self.state;

        let grid = sweeten::tile_grid(state, |id, item| {
            let is_focused = focus == Some(id);

            let title = text!("Item {}", item.id)
                .size(13)
                .style(style::title_text)
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

            let node = state.get_item(id);
            let (gx, gy, gw, gh) =
                node.map(|i| (i.x, i.y, i.w, i.h)).unwrap_or((0, 0, 0, 0));

            let can_interact = !item.is_pinned && !self.locked_all;

            grid_content(view_content(gx, gy, gw, gh))
                .title_bar(title_bar)
                .draggable(can_interact)
                .resizable(can_interact)
                .held(item.is_pinned)
                .style(if is_focused {
                    style::item_focused
                } else {
                    style::item_active
                })
        })
        .width(Fill)
        .height(Shrink)
        .spacing(8)
        .on_action(Message::GridAction)
        .locked(self.locked_all)
        .style(style::grid_style);

        let add_button = button(text("+ Add Item").size(13))
            .on_press(Message::AddItem)
            .style(button::primary)
            .padding([6, 16]);

        let lock_icon = if self.locked_all {
            icon::lock().size(14)
        } else {
            icon::lock_open().size(14)
        };
        let lock_button = button(lock_icon)
            .on_press(Message::ToggleLockAll)
            .style(button::text)
            .padding([4, 8]);

        let toolbar = container(
            row![
                add_button,
                text!(
                    "{} items  |  {}-column  |  Cmd+N / Cmd+Shift+N / Cmd+W  |  Shift: place",
                    total_items,
                    state.columns()
                )
                .size(12)
                .style(style::muted_text),
                space::horizontal(),
                lock_button,
            ]
            .spacing(16)
            .align_y(Center),
        )
        .padding([10, 14]);

        let grid_area = scrollable(
            container(grid).padding(10).style(style::grid_background),
        )
        .height(Fill);

        column![toolbar, rule::horizontal(1), grid_area].into()
    }
}

/// Focus the item before `id`, or the last item if none precede it.
fn prev_or_last_item<T>(
    state: &tile_grid::State<T>,
    id: tile_grid::ItemId,
) -> Option<tile_grid::ItemId> {
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
                .style(style::label_text),
            text!("Size  {} x {}", gw, gh)
                .center()
                .size(13)
                .style(style::label_text),
        ]
        .spacing(4)
        .width(Fill)
        .align_x(Center),
    )
    .padding(10)
    .into()
}

fn view_controls(
    id: tile_grid::ItemId,
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
    use iced::widget::{container, text};
    use iced::{Border, Theme};

    use sweeten::widget::tile_grid;

    const RADIUS: f32 = 6.0;

    pub fn title_text(theme: &Theme) -> text::Style {
        text::Style {
            color: Some(theme.palette().background.base.text),
        }
    }

    pub fn label_text(theme: &Theme) -> text::Style {
        text::Style {
            color: Some(theme.palette().background.strong.text),
        }
    }

    pub fn muted_text(theme: &Theme) -> text::Style {
        text::Style {
            color: Some(theme.palette().background.weak.text),
        }
    }

    pub fn title_bar_active(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            text_color: Some(palette.background.base.text),
            background: Some(palette.background.base.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            text_color: Some(palette.background.base.text),
            background: Some(palette.background.base.color.into()),
            ..Default::default()
        }
    }

    pub fn item_active(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: Border {
                width: 1.0,
                color: palette.background.weak.color,
                radius: RADIUS.into(),
            },
            ..Default::default()
        }
    }

    pub fn item_focused(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: Border {
                width: 2.0,
                color: palette.primary.base.color,
                radius: RADIUS.into(),
            },
            ..Default::default()
        }
    }

    pub fn grid_background(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.weakest.color.into()),
            border: Border {
                width: 1.0,
                color: palette.background.weak.color,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }

    /// Custom `TileGrid` style that tints the ghost with the theme's
    /// warning color while the user is dragging in Place mode
    /// (Shift held).
    pub fn grid_style(theme: &Theme) -> tile_grid::Style {
        let place_color = theme.palette().warning.base.color;

        tile_grid::Style {
            place_region: Some(tile_grid::Highlight {
                background: place_color.scale_alpha(0.16).into(),
                border: Border {
                    width: 2.0,
                    color: place_color,
                    radius: RADIUS.into(),
                },
            }),
            ..tile_grid::default_style(theme)
        }
    }
}
