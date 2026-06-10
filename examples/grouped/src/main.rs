//! A grouped `tile_grid` dashboard with an edit mode.
//!
//! Read-only by default. Click **Customize** to enter edit mode, which frames
//! every group with a border and — on hover — reveals controls (straddling
//! overlays pinned to the hovered item's top-right corner):
//!
//! - tiles get an **edit** control (opens a modal) and a **delete** control;
//! - groups get a tinted **+ Add tile** command and a **delete** control, and
//!   their title becomes an inline `text_input` for renaming.
//!
//! Deletion is a two-step *arm-and-execute*: the first click arms (the icon
//! turns into a red check); the second confirms. Moving/resizing is enabled
//! only in edit mode. Groups are sized to their children, so adding,
//! removing, or dragging tiles reflows the layout.
//!
//! Run with: `cargo run -p grouped`

#[allow(dead_code)]
mod icon;

use iced::widget::{
    button, center, column, container, mouse_area, opaque, row, space, stack,
    text, text_input,
};
use iced::{
    Background, Border, Center, Color, Element, Fill, Font, Task, Theme, color,
};

use sweeten::widget::tile_grid::{
    Action, CellHeight, ItemId, State, Width, default_style, grid_content,
    title_bar,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("sweeten • grouped tile_grid")
        .theme(theme)
        .font(icon::FONT)
        .window_size((1100.0, 760.0))
        .run()
}

fn theme(_state: &App) -> Theme {
    Theme::Light
}

/// What a node carries. A node's *role* (container vs leaf) comes from the
/// tree; this only tells the view closure how to render it.
#[derive(Clone)]
enum Cell {
    /// A section header (group label).
    Section(String),
    /// A leaf tile: an editable title and headline value.
    Tile { title: String, value: String },
}

/// An in-progress edit, shown as a modal. `value` is `None` for a section
/// (only its label is editable).
struct Edit {
    id: ItemId,
    title: String,
    value: Option<String>,
}

struct App {
    state: State<Cell>,
    edit_mode: bool,
    /// The item whose delete control is armed (awaiting confirmation).
    armed: Option<ItemId>,
    editing: Option<Edit>,
}

#[derive(Debug, Clone)]
enum Message {
    Grid(Action),
    ToggleEdit,
    StartEdit(ItemId),
    EditTitle(String),
    EditValue(String),
    SaveEdit,
    CancelEdit,
    ArmDelete(ItemId),
    ConfirmDelete(ItemId),
    AddTile(ItemId),
    GroupTitle(ItemId, String),
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

        // Groups are authored 1 row tall; `size_to_content` grows them to
        // fit their children.
        let pulse = state.add_group(
            [0, 0, 8, 1],
            Width::Shrink,
            Cell::Section("Pulse".into()),
        );
        for (x, t, v) in [
            (0, "New Bookings", "$4.2M"),
            (2, "Pipeline Coverage", "3.4×"),
            (4, "Win Rate", "23.8%"),
            (6, "Quota Attainment", "78%"),
        ] {
            state.add_child(pulse, [x, 0, 2, 2], tile(t, v));
        }

        let trends = state.add_group(
            [0, 1, 8, 1],
            Width::Shrink,
            Cell::Section("Trends".into()),
        );
        state.add_child(
            trends,
            [0, 0, 4, 3],
            tile("Bookings vs Plan", "▁▂▃▅▆▇"),
        );
        state.add_child(
            trends,
            [4, 0, 4, 3],
            tile("Pipeline by Stage", "▇▆▅▃▂"),
        );

        // A Shrink group: its tiles can grow and the rail grows with them.
        let rail = state.add_group(
            [8, 0, 4, 1],
            Width::Shrink,
            Cell::Section(String::new()),
        );
        state.add_child(rail, [0, 0, 4, 3], tile("News Feed", "3 new"));
        state.add_child(rail, [0, 3, 4, 3], tile("Markets", "S&P 7,580"));

        // A standalone (ungrouped) 2x2 tile on the root board, gravity-packed
        // below the news/markets rail. Useful for exercising drags through the
        // busy intersection between the root grid and the groups.
        state.add([8, 1000, 2, 2], tile("Loose tile", "—"));

        // Size groups to their content. This commits each group's width to
        // the columns its tiles occupy, so a group whose tiles shrink keeps a
        // snug footprint and can be dragged flush to the board edges rather
        // than being held back by the width it was authored with. The widget
        // reads this flag off the state.
        state.fit(true);

        (
            Self {
                state,
                edit_mode: false,
                armed: None,
                editing: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Grid(action) => {
                self.state.perform(action, |_, _| false);
            }
            Message::ToggleEdit => {
                self.edit_mode = !self.edit_mode;
                self.armed = None;
            }
            Message::StartEdit(id) => {
                self.armed = None;
                self.editing = match self.state.get(id) {
                    Some(Cell::Tile { title, value }) => Some(Edit {
                        id,
                        title: title.clone(),
                        value: Some(value.clone()),
                    }),
                    Some(Cell::Section(label)) => Some(Edit {
                        id,
                        title: label.clone(),
                        value: None,
                    }),
                    None => None,
                };
            }
            Message::EditTitle(title) => {
                if let Some(edit) = &mut self.editing {
                    edit.title = title;
                }
            }
            Message::EditValue(value) => {
                if let Some(edit) = &mut self.editing
                    && edit.value.is_some()
                {
                    edit.value = Some(value);
                }
            }
            Message::SaveEdit => {
                if let Some(edit) = self.editing.take()
                    && let Some(cell) = self.state.get_mut(edit.id)
                {
                    match cell {
                        Cell::Tile { title, value } => {
                            *title = edit.title;
                            if let Some(v) = edit.value {
                                *value = v;
                            }
                        }
                        Cell::Section(label) => *label = edit.title,
                    }
                }
            }
            Message::CancelEdit => self.editing = None,
            Message::ArmDelete(id) => self.armed = Some(id),
            Message::ConfirmDelete(id) => {
                self.state.remove(id);
                self.armed = None;
            }
            Message::AddTile(group) => {
                // Drop the new tile below everything; gravity compacts it
                // into the first free slot, and size-to-content grows the
                // group to fit.
                self.state.add_child(
                    group,
                    [0, 1000, 2, 2],
                    tile("New tile", "—"),
                );
            }
            Message::GroupTitle(id, label) => {
                if let Some(Cell::Section(section)) = self.state.get_mut(id) {
                    *section = label;
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let edit_mode = self.edit_mode;
        let armed = self.armed;

        let grid =
            sweeten::tile_grid(&self.state, move |id, cell| match cell {
                Cell::Section(label) => {
                    // In edit mode every group shows its title (an inline,
                    // editable input); read-only hides empty labels.
                    let has_header = edit_mode || !label.is_empty();
                    let mut content = grid_content(
                        container(text("")).width(Fill).height(Fill),
                    )
                    .resizable(false);
                    if has_header {
                        // Center the title in its strip (the widget frames the
                        // strip with `group_padding` top and bottom) and let it
                        // align with the tiles' left edge — so the title bar
                        // carries no padding of its own.
                        content = content.title_bar(title_bar(
                            container(group_title(id, label, edit_mode))
                                .center_y(Fill),
                        ));
                    } else {
                        content = content.draggable(false);
                    }
                    if edit_mode {
                        // Drag the group from anywhere on its body, not just
                        // the title bar.
                        content = content.drag_body(true).controls(
                            row![add_tile_button(id), delete_button(id, armed)]
                                .spacing(4),
                        );
                    }
                    content
                }
                Cell::Tile { title, value } => {
                    let mut content = grid_content(
                        center(text(value.clone()).size(22)).padding(14),
                    )
                    .title_bar(
                        title_bar(text(title.clone()).size(12).style(muted))
                            .padding([6, 8]),
                    )
                    .style(tile_style);
                    if edit_mode {
                        content = content.drag_body(true).controls(
                            row![edit_button(id), delete_button(id, armed)]
                                .spacing(4),
                        );
                    }
                    content
                }
            })
            .width(Fill)
            .height(Fill)
            .spacing(8)
            .cell_height(CellHeight::Fixed(54.0))
            .group_header(24)
            .locked(!edit_mode)
            .style(move |theme| {
                let mut style = default_style(theme);
                // Fill groups with the board's off-white so the drag ghost is
                // opaque rather than see-through.
                style.group_background =
                    Some(Background::Color(color!(0xf4f4f5)));
                style.hovered_region.border.radius = 8.0.into();
                if edit_mode {
                    style.group_border = Some(Border {
                        width: 1.0,
                        color: theme.palette().background.strong.color,
                        radius: 8.0.into(),
                    });
                }
                style
            })
            .on_action(Message::Grid);

        let toolbar = row![
            text("Dashboard").size(20).font(Font {
                weight: iced::font::Weight::Bold,
                ..Font::DEFAULT
            }),
            space::horizontal(),
            customize_button(self.edit_mode),
        ]
        .align_y(Center)
        .padding([4, 4]);

        let base = container(column![toolbar, grid].spacing(12))
            .padding(16)
            .width(Fill)
            .height(Fill)
            // Off-white board (Tailwind zinc-100) so the white tiles stand out.
            .style(|_| container::Style {
                background: Some(Background::Color(color!(0xf4f4f5))),
                ..container::Style::default()
            });

        match &self.editing {
            None => base.into(),
            Some(edit) => modal(base, edit_form(edit), Message::CancelEdit),
        }
    }
}

// ── controls ────────────────────────────────────────────────────

fn customize_button(edit_mode: bool) -> Element<'static, Message> {
    let (glyph, label) = if edit_mode {
        (icon::check(), "Done")
    } else {
        (icon::pencil(), "Customize")
    };
    button(
        row![glyph.size(14), text(label).size(13)]
            .spacing(6)
            .align_y(Center),
    )
    .on_press(Message::ToggleEdit)
    .padding([6, 12])
    .style(outlined)
    .into()
}

/// The group's title: static text when read-only, an inline editable
/// `text_input` (with a placeholder) when in edit mode — styled to be
/// visually identical to the static text.
fn group_title(
    id: ItemId,
    label: &str,
    edit_mode: bool,
) -> Element<'_, Message> {
    if edit_mode {
        text_input("Enter a title", label)
            .size(16)
            .padding(0)
            .width(Fill)
            .on_input(move |value| Message::GroupTitle(id, value))
            .style(title_input)
            .into()
    } else {
        text(label.to_owned()).size(16).style(section_label).into()
    }
}

fn edit_button(id: ItemId) -> Element<'static, Message> {
    button(icon::square_pen().size(13))
        .on_press(Message::StartEdit(id))
        .padding([2, 6])
        .style(pill)
        .into()
}

fn add_tile_button(group: ItemId) -> Element<'static, Message> {
    button(
        row![icon::plus().size(13), text("Add tile").size(11)]
            .spacing(4)
            .align_y(Center),
    )
    .on_press(Message::AddTile(group))
    .padding([2, 8])
    .style(tinted)
    .into()
}

/// The delete control: a trash icon that arms on first click and confirms
/// (red check) on the second.
fn delete_button(
    id: ItemId,
    armed: Option<ItemId>,
) -> Element<'static, Message> {
    if armed == Some(id) {
        button(icon::check().size(13))
            .on_press(Message::ConfirmDelete(id))
            .padding([2, 6])
            .style(danger)
            .into()
    } else {
        button(icon::trash().size(13))
            .on_press(Message::ArmDelete(id))
            .padding([2, 6])
            .style(pill)
            .into()
    }
}

/// The tile/section edit form shown inside the modal.
fn edit_form(edit: &Edit) -> Element<'_, Message> {
    let title_field = column![
        text(if edit.value.is_some() {
            "Title"
        } else {
            "Label"
        })
        .size(12)
        .style(muted),
        text_input("Title", &edit.title)
            .on_input(Message::EditTitle)
            .on_submit(Message::SaveEdit)
            .padding(6),
    ]
    .spacing(4);

    let value_field = edit.value.as_ref().map(|value| {
        column![
            text("Value").size(12).style(muted),
            text_input("Value", value)
                .on_input(Message::EditValue)
                .on_submit(Message::SaveEdit)
                .padding(6),
        ]
        .spacing(4)
    });

    let mut form = column![text("Edit").size(18), title_field].spacing(14);
    if let Some(value_field) = value_field {
        form = form.push(value_field);
    }
    form = form.push(
        row![
            button(text("Cancel"))
                .on_press(Message::CancelEdit)
                .style(button::secondary),
            button(text("Save")).on_press(Message::SaveEdit),
        ]
        .spacing(8),
    );

    container(form)
        .width(320)
        .padding(16)
        .style(container::rounded_box)
        .into()
}

/// Stacks `content` over `base` behind a dimmed, click-to-dismiss backdrop.
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

// ── styles ──────────────────────────────────────────────────────

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

/// An outlined (transparent-fill) button, as in the mockup's "Customize".
fn outlined(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let background =
        matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| palette.background.weak.color.into());
    button::Style {
        background,
        text_color: palette.background.base.text,
        border: Border {
            width: 1.0,
            color: palette.background.strong.color,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    }
}

/// A `text_input` styled to be visually identical to the group's title
/// text: transparent, borderless, with the title's color and a muted
/// placeholder.
fn title_input(
    theme: &Theme,
    _status: text_input::Status,
) -> text_input::Style {
    let palette = theme.palette();
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: palette.background.weak.color,
        placeholder: palette.background.strong.color,
        value: palette.background.base.text,
        selection: palette.primary.weak.color,
    }
}

fn tile_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    container::Style {
        // White fill so tiles read as cards on the off-white board.
        background: Some(Background::Color(Color::WHITE)),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn pill(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            palette.background.weak.color
        }
        _ => palette.background.weakest.color,
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

fn tinted(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let strong =
        matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if strong {
                palette.primary.base.color
            } else {
                palette.primary.weak.color
            }
            .into(),
        ),
        text_color: palette.primary.strong.text,
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}

fn danger(theme: &Theme, _status: button::Status) -> button::Style {
    let palette = theme.palette();
    button::Style {
        background: Some(palette.danger.base.color.into()),
        text_color: palette.danger.base.text,
        border: Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..button::Style::default()
    }
}
