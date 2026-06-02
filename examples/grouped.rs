//! A grouped `tile_grid` dashboard: labeled sections ("Pulse", "Trends")
//! and an unlabeled right rail. Tiles can be dragged *within* a section or
//! *across* sections (every container body is outlined while dragging), and
//! sections rearrange by their header.
//!
//! Each tile carries an edit control pinned to its top-right corner; clicking
//! it opens a modal (the `stack`/`opaque`/`center` pattern from iced's
//! `modal` example) to edit the tile's title and value in place.
//!
//! Run with: `cargo run --example grouped`

use iced::widget::{
    button, center, column, container, mouse_area, opaque, row, stack, text,
    text_input,
};
use iced::{Border, Color, Element, Fill, Task, Theme};

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

/// What a node carries. A node's *role* (container vs leaf) comes from the
/// tree, not this data — but we tag each node so the view closure knows how
/// to render it.
#[derive(Clone)]
enum Cell {
    /// A section header. An empty label renders no header (an unlabeled
    /// group, like the right rail).
    Section(&'static str),
    /// A leaf tile: an editable title and headline value.
    Tile { title: String, value: String },
}

/// An in-progress tile edit, shown as a modal.
struct Edit {
    id: ItemId,
    title: String,
    value: String,
}

struct App {
    state: State<Cell>,
    focus: Option<ItemId>,
    editing: Option<Edit>,
}

#[derive(Debug, Clone)]
enum Message {
    Grid(Action),
    StartEdit(ItemId),
    EditTitle(String),
    EditValue(String),
    SaveEdit,
    CancelEdit,
}

fn tile(title: &str, value: &str) -> Cell {
    Cell::Tile {
        title: title.to_owned(),
        value: value.to_owned(),
    }
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut state: State<Cell> = State::new(12);

        // Left column: "Pulse" over "Trends". Right rail beside them.
        let pulse = state.add_group(0, 0, 8, 3, Cell::Section("Pulse"), 8);
        for (x, t, v) in [
            (0, "New Bookings", "$4.2M"),
            (2, "Pipeline Coverage", "3.4×"),
            (4, "Win Rate", "23.8%"),
            (6, "Quota Attainment", "78%"),
        ] {
            state.add_child(pulse, x, 0, 2, 2, tile(t, v));
        }

        let trends = state.add_group(0, 3, 8, 4, Cell::Section("Trends"), 8);
        state.add_child(trends, 0, 0, 4, 3, tile("Bookings vs Plan", "▁▂▃▅▆▇"));
        state.add_child(trends, 4, 0, 4, 3, tile("Pipeline by Stage", "▇▆▅▃▂"));

        // Unlabeled right rail (no header): News + Markets, stacked.
        let rail = state.add_group(8, 0, 4, 7, Cell::Section(""), 1);
        state.add_child(rail, 0, 0, 1, 3, tile("News Feed", "3 new"));
        state.add_child(rail, 0, 3, 1, 3, tile("Markets", "S&P 7,580"));

        (
            Self {
                state,
                focus: None,
                editing: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Grid(action) => {
                if action.is_click() {
                    self.focus = Some(action.id());
                }
                self.state.perform(action, |_, _| false);
            }
            Message::StartEdit(id) => {
                if let Some(Cell::Tile { title, value }) = self.state.get(id) {
                    self.editing = Some(Edit {
                        id,
                        title: title.clone(),
                        value: value.clone(),
                    });
                }
            }
            Message::EditTitle(title) => {
                if let Some(edit) = &mut self.editing {
                    edit.title = title;
                }
            }
            Message::EditValue(value) => {
                if let Some(edit) = &mut self.editing {
                    edit.value = value;
                }
            }
            Message::SaveEdit => {
                if let Some(edit) = self.editing.take()
                    && let Some(Cell::Tile { title, value }) =
                        self.state.get_mut(edit.id)
                {
                    *title = edit.title;
                    *value = edit.value;
                }
            }
            Message::CancelEdit => {
                self.editing = None;
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

                    // The edit control sits in the title bar's controls slot,
                    // pinned to the tile's top-right corner (and excluded from
                    // the drag pick area, so clicking it never starts a drag).
                    let edit = button(text("Edit").size(10))
                        .on_press(Message::StartEdit(id))
                        .padding([1, 6])
                        .style(pill);

                    grid_content(
                        center(text(value.clone()).size(22)).padding(16),
                    )
                    .title_bar(
                        title_bar(text(title.clone()).size(12).style(muted))
                            .controls(edit)
                            .always_show_controls()
                            .padding([6, 8]),
                    )
                    .style(if is_focused {
                        tile_focused
                    } else {
                        tile_style
                    })
                }
            })
            .width(Fill)
            .height(Fill)
            .spacing(8)
            .cell_height(CellHeight::Fixed(56.0))
            .group_header(30)
            .on_action(Message::Grid);

        let base = container(grid).padding(16);

        match &self.editing {
            None => base.into(),
            Some(edit) => modal(base, edit_form(edit), Message::CancelEdit),
        }
    }
}

/// The tile-edit form shown inside the modal.
fn edit_form(edit: &Edit) -> Element<'_, Message> {
    container(
        column![
            text("Edit tile").size(18),
            column![
                text("Title").size(12).style(muted),
                text_input("Title", &edit.title)
                    .on_input(Message::EditTitle)
                    .on_submit(Message::SaveEdit)
                    .padding(6),
            ]
            .spacing(4),
            column![
                text("Value").size(12).style(muted),
                text_input("Value", &edit.value)
                    .on_input(Message::EditValue)
                    .on_submit(Message::SaveEdit)
                    .padding(6),
            ]
            .spacing(4),
            row![
                button(text("Cancel"))
                    .on_press(Message::CancelEdit)
                    .style(button::secondary),
                button(text("Save")).on_press(Message::SaveEdit),
            ]
            .spacing(8),
        ]
        .spacing(14),
    )
    .width(320)
    .padding(16)
    .style(container::rounded_box)
    .into()
}

/// Stacks `content` over `base` behind a dimmed, click-to-dismiss backdrop.
/// Adapted from iced's `modal` example.
fn modal<'a>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message> {
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.6,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
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

fn tile_style(theme: &Theme) -> container::Style {
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
            ..tile_style(theme).border
        },
        ..tile_style(theme)
    }
}

/// A compact, rounded "pill" style for the tile's edit control.
fn pill(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            palette.primary.weak.color
        }
        _ => palette.background.weak.color,
    };
    button::Style {
        background: Some(background.into()),
        text_color: palette.background.base.text,
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}
