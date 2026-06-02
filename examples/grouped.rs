//! A grouped `tile_grid` dashboard: labeled sections ("Pulse", "Trends")
//! and an unlabeled right rail, with tiles that can be dragged *within* a
//! section or *across* sections, and sections that can be rearranged by
//! their header.
//!
//! Run with: `cargo run --example grouped`

use iced::widget::{center, column, container, text};
use iced::{Border, Center, Element, Fill, Task, Theme};

use sweeten::widget::tile_grid::{
    Action, CellHeight, ItemId, State, grid_content, title_bar,
};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("sweeten • grouped tile_grid")
        .theme(theme)
        .window_size((1100.0, 760.0))
        .run()
}

fn theme(_state: &App) -> Theme {
    Theme::Light
}

/// What a node carries. A node's *role* (container vs leaf) is determined by
/// the tree, not this data — but we still tag each node so the view closure
/// knows how to render it.
#[derive(Clone)]
enum Cell {
    /// A section header. An empty label renders no header (an unlabeled
    /// group, like the right rail).
    Section(&'static str),
    /// A leaf tile: a title and a headline value.
    Tile {
        title: &'static str,
        value: &'static str,
    },
}

struct App {
    state: State<Cell>,
    focus: Option<ItemId>,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Grid(Action),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut state: State<Cell> = State::new(12);

        // Left column: "Pulse" over "Trends". Right rail beside them.
        let pulse = state.add_group(0, 0, 8, 3, Cell::Section("Pulse"), 8);
        for (x, title, value) in [
            (0, "New Bookings", "$4.2M"),
            (2, "Pipeline Coverage", "3.4×"),
            (4, "Win Rate", "23.8%"),
            (6, "Quota Attainment", "78%"),
        ] {
            state.add_child(pulse, x, 0, 2, 2, Cell::Tile { title, value });
        }

        let trends = state.add_group(0, 3, 8, 4, Cell::Section("Trends"), 8);
        state.add_child(
            trends,
            0,
            0,
            4,
            3,
            Cell::Tile {
                title: "Bookings — Actual vs Plan",
                value: "▁▂▃▅▆▇",
            },
        );
        state.add_child(
            trends,
            4,
            0,
            4,
            3,
            Cell::Tile {
                title: "Pipeline by Stage",
                value: "▇▆▅▃▂",
            },
        );

        // Unlabeled right rail (no header): News + Markets, stacked.
        let rail = state.add_group(8, 0, 4, 7, Cell::Section(""), 1);
        state.add_child(
            rail,
            0,
            0,
            1,
            3,
            Cell::Tile {
                title: "News Feed",
                value: "3 new",
            },
        );
        state.add_child(
            rail,
            0,
            3,
            1,
            3,
            Cell::Tile {
                title: "Markets",
                value: "S&P 7,580",
            },
        );

        (Self { state, focus: None }, Task::none())
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Grid(action) => {
                if action.is_click() {
                    self.focus = Some(action.id());
                }
                self.state.perform(action, |_, _| false);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let focus = self.focus;

        let grid =
            sweeten::tile_grid(&self.state, move |id, cell| match cell {
                Cell::Section(label) => {
                    let body = grid_content(
                        container(text("")).width(Fill).height(Fill),
                    );
                    if label.is_empty() {
                        // Unlabeled container (the rail): no header, not
                        // draggable as a unit.
                        body.draggable(false).resizable(false)
                    } else {
                        body.title_bar(
                            title_bar(
                                text(*label).size(16).style(section_label),
                            )
                            .padding([4, 6]),
                        )
                        .resizable(false)
                    }
                }
                Cell::Tile { title, value } => {
                    let is_focused = focus == Some(id);
                    grid_content(
                        center(
                            column![
                                text(*title).size(12).style(muted),
                                text(*value).size(22),
                            ]
                            .spacing(4)
                            .align_x(Center),
                        )
                        .padding(8),
                    )
                    .title_bar(title_bar(text(*title).size(11).style(muted)))
                    .style(if is_focused {
                        tile_focused
                    } else {
                        tile
                    })
                }
            })
            .width(Fill)
            .height(Fill)
            .spacing(8)
            .cell_height(CellHeight::Fixed(56.0))
            .group_header(30)
            .on_action(Message::Grid);

        container(grid).padding(16).into()
    }
}

fn section_label(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().background.base.text),
    }
}

fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().background.strong.color),
    }
}

fn tile(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(palette.background.base.color.into()),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn tile_focused(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        border: Border {
            width: 2.0,
            color: palette.primary.base.color,
            ..tile(theme).border
        },
        ..tile(theme)
    }
}
