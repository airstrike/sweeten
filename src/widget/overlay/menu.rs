//! Build and show dropdown menus.
use crate::core::alignment;
use crate::core::border::{self, Border};
use crate::core::keyboard;
use crate::core::keyboard::key;
use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::text::paragraph;
use crate::core::text::{self, Text};
use crate::core::touch;
use crate::core::widget::Id;
use crate::core::widget::Meta;
use crate::core::widget::operation;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    Background, Color, Event, Font, Length, Padding, Pixels, Point, Rectangle,
    Shadow, Size, Theme, Vector,
};
use crate::core::{Element, Shell, Widget};
use crate::scrollable::{self, Scrollable};
use crate::widget::pick_list::{Anchor, Content, Entry, Options};

use std::convert::Infallible;

use std::time::{Duration, Instant};

// --- sweeten: disabled items, entries, rich content, keyboard support ---

/// The time window during which consecutive keystrokes accumulate into a
/// typeahead query instead of starting a new one.
const TYPEAHEAD_TIMEOUT: Duration = Duration::from_millis(700);

/// The default text size of a group label, relative to the text size of
/// the options of the menu.
const LABEL_TEXT_RATIO: f32 = 0.85;

/// How often the menu scrolls one row while a scroll chevron is hovered.
const AUTO_SCROLL_INTERVAL: Duration = Duration::from_millis(50);

/// The breathing room left around a row scrolled into view by keyboard
/// navigation.
const SCROLL_MARGIN: f32 = 4.0;

/// How much sharper the highlights are than the menu box — the radius
/// step between a menu and its items in common design systems.
const HIGHLIGHT_RADIUS_STEP: f32 = 2.0;

/// A list of selectable options.
pub struct Menu<
    'a,
    'b,
    T,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    state: &'a mut State,
    options: &'a mut Options<'b, T, Theme, Renderer>,
    disabled: Option<Vec<bool>>,
    hovered_option: &'a mut Option<usize>,
    view: &'a dyn Fn(&T) -> Content<'b, Theme, Renderer>,
    typeahead: Option<&'a dyn Fn(&T) -> String>,
    on_selected: Box<dyn FnMut(Option<T>) -> Message + 'a>,
    on_option_hovered: Option<&'a dyn Fn(T) -> Message>,
    width: f32,
    menu_width: Option<Length>,
    height: Length,
    selected: Option<usize>,
    check_indicator: bool,
    anchor: Anchor,
    separator: bool,
    menu_padding: Padding,
    target_radius: Option<border::Radius>,
    padding: Padding,
    text_size: Option<Pixels>,
    line_height: Option<text::LineHeight>,
    shaping: text::Shaping,
    ellipsis: text::Ellipsis,
    font: Option<Font>,
    class: &'a <Theme as Catalog>::Class<'b>,
}

impl<'a, 'b, T, Message, Theme, Renderer>
    Menu<'a, 'b, T, Message, Theme, Renderer>
where
    T: Clone,
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'a,
    'b: 'a,
{
    /// Creates a new [`Menu`] with the given [`State`], some [`Options`],
    /// a function producing the [`Content`] displayed for each option, the
    /// message to produce when an option is selected, and its [`Style`].
    ///
    /// The `disabled` list applies to selectable options, in order; an
    /// option is disabled if either its [`Entry`] or the list says so.
    /// The `typeahead` function provides the text matched against
    /// keyboard input for options whose [`Content`] is an element.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: &'a mut State,
        options: &'a mut Options<'b, T, Theme, Renderer>,
        hovered_option: &'a mut Option<usize>,
        view: &'a dyn Fn(&T) -> Content<'b, Theme, Renderer>,
        on_selected: impl FnMut(Option<T>) -> Message + 'a,
        disabled: Option<Vec<bool>>,
        typeahead: Option<&'a dyn Fn(&T) -> String>,
        on_option_hovered: Option<&'a dyn Fn(T) -> Message>,
        class: &'a <Theme as Catalog>::Class<'b>,
    ) -> Self {
        Menu {
            state,
            options,
            disabled,
            hovered_option,
            view,
            typeahead,
            on_selected: Box::new(on_selected),
            on_option_hovered,
            width: 0.0,
            menu_width: None,
            height: Length::Shrink,
            selected: None,
            check_indicator: false,
            anchor: Anchor::default(),
            separator: false,
            menu_padding: Padding::ZERO,
            target_radius: None,
            padding: Padding::ZERO,
            text_size: None,
            line_height: None,
            shaping: text::Shaping::default(),
            ellipsis: text::Ellipsis::default(),
            font: None,
            class,
        }
    }

    /// Sets the width of the target of the [`Menu`].
    ///
    /// This is the width the [`Menu`] takes, unless overridden with
    /// [`Menu::menu_width`].
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the width of the [`Menu`].
    ///
    /// By default — and with [`Length::Shrink`] — the menu fits its widest
    /// entry, but never gets narrower than its target.
    pub fn menu_width(mut self, menu_width: impl Into<Length>) -> Self {
        self.menu_width = Some(menu_width.into());
        self
    }

    /// Sets the height of the [`Menu`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the selected option of the [`Menu`], by index.
    ///
    /// The index counts selectable options only, skipping labels and
    /// separators.
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Sets whether the [`Menu`] displays a check indicator next to the
    /// selected option.
    pub fn check_indicator(mut self, check_indicator: bool) -> Self {
        self.check_indicator = check_indicator;
        self
    }

    /// Sets the [`Anchor`] of the [`Menu`] relative to its target.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Sets whether a horizontal rule separates the groups of the
    /// [`Menu`], drawn centered in the [`Options::spacing`] gap.
    pub fn separator(mut self, separator: bool) -> Self {
        self.separator = separator;
        self
    }

    /// Sets the inner [`Padding`] of the [`Menu`], inset between its
    /// border and its contents.
    pub fn menu_padding(mut self, menu_padding: impl Into<Padding>) -> Self {
        self.menu_padding = menu_padding.into();
        self
    }

    /// Sets the border radius of the target of the [`Menu`].
    ///
    /// When set, the menu border and its highlights follow this radius
    /// instead of the radius of the menu [`Style`], so the whole control
    /// shares one look.
    pub fn target_radius(
        mut self,
        target_radius: impl Into<border::Radius>,
    ) -> Self {
        self.target_radius = Some(target_radius.into());
        self
    }

    /// Sets the [`Padding`] of the [`Menu`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the text size of the [`Menu`].
    pub fn text_size(mut self, text_size: impl Into<Pixels>) -> Self {
        self.text_size = Some(text_size.into());
        self
    }

    /// Sets the text [`text::LineHeight`] of the [`Menu`].
    pub fn line_height(
        mut self,
        line_height: impl Into<text::LineHeight>,
    ) -> Self {
        self.line_height = Some(line_height.into());
        self
    }

    /// Sets the [`text::Shaping`] strategy of the [`Menu`].
    pub fn shaping(mut self, shaping: text::Shaping) -> Self {
        self.shaping = shaping;
        self
    }

    /// Sets the [`text::Ellipsis`] strategy of the [`Menu`].
    pub fn ellipsis(mut self, ellipsis: text::Ellipsis) -> Self {
        self.ellipsis = ellipsis;
        self
    }

    /// Sets the font of the [`Menu`].
    pub fn font(mut self, font: impl Into<Font>) -> Self {
        self.font = Some(font.into());
        self
    }

    /// Check if the option at the given index is disabled.
    ///
    /// The index counts selectable options only, skipping labels and
    /// separators.
    pub fn is_disabled(&self, index: usize) -> bool {
        let inline = self
            .options
            .groups()
            .iter()
            .flat_map(|group| group.entries.iter())
            .map(|entry| match entry {
                Entry::Option { disabled, .. } => *disabled,
                Entry::Deselect(_) => false,
            })
            .nth(index)
            .unwrap_or(false);

        inline
            || self
                .disabled
                .as_ref()
                .and_then(|disabled| disabled.get(index))
                .copied()
                .unwrap_or(false)
    }

    /// Turns the [`Menu`] into an [`overlay::Element`] at the given target
    /// position.
    ///
    /// `position` is the target's position in screen coordinates, and
    /// `window` is the size of the window.
    ///
    /// The `target_height` will be used to display the menu either on top
    /// of the target or under it, depending on the screen position and the
    /// dimensions of the [`Menu`].
    pub fn overlay(
        self,
        renderer: &Renderer,
        position: Point,
        window: Size,
        target_height: f32,
    ) -> overlay::Element<'a, Message, Theme, Renderer>
    where
        Renderer: text::Renderer + 'a,
    {
        overlay::Element::new(Box::new(Overlay::new(
            position,
            window,
            self,
            target_height,
            renderer,
        )))
    }
}

/// The local state of a [`Menu`].
#[derive(Debug)]
pub struct State {
    tree: Tree,
}

impl State {
    /// Creates a new [`State`] for a [`Menu`].
    pub fn new() -> Self {
        Self {
            tree: Tree::empty(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

struct Overlay<'a, 'b, T, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    window: Size,
    /// The whole menu box, chevron strips included.
    layout: Layout,
    /// The scrollable list, inset between the chevron strips.
    list_layout: Layout,
    tree: &'a mut Tree,
    list: Scrollable<
        'a,
        Message,
        List<'a, 'b, T, Message, Theme, Renderer>,
        Theme,
    >,
    id: Id,
    target_radius: Option<border::Radius>,
    class: &'a <Theme as Catalog>::Class<'b>,
}

impl<'a, 'b, T, Message, Theme, Renderer>
    Overlay<'a, 'b, T, Message, Theme, Renderer>
where
    T: Clone,
    Message: 'a,
    Theme: Catalog + scrollable::Catalog + 'a,
    Renderer: text::Renderer + 'a,
    'b: 'a,
{
    pub fn new(
        position: Point,
        window: Size,
        menu: Menu<'a, 'b, T, Message, Theme, Renderer>,
        target_height: f32,
        renderer: &Renderer,
    ) -> Self {
        let Menu {
            state,
            options,
            disabled,
            hovered_option,
            view,
            typeahead,
            on_selected,
            on_option_hovered,
            width,
            menu_width,
            height,
            selected,
            check_indicator,
            anchor,
            separator,
            menu_padding,
            target_radius,
            padding,
            font,
            text_size,
            line_height,
            shaping,
            ellipsis,
            class,
        } = menu;

        let spacing = options.spacing;

        let mut option_rows = Vec::new();
        let mut values = Vec::new();
        let mut rows = Vec::new();

        for (group_index, group) in options.groups.iter_mut().enumerate() {
            if group_index > 0 {
                rows.push(Row {
                    kind: RowKind::Divider,
                    option_index: None,
                    disabled: true,
                    search: None,
                });
            }

            match &mut group.title {
                Some(Content::Text(title)) => rows.push(Row {
                    kind: RowKind::Title(title.clone()),
                    option_index: None,
                    disabled: true,
                    search: None,
                }),
                Some(Content::Element(element)) => rows.push(Row {
                    kind: RowKind::ElementRef(element),
                    option_index: None,
                    disabled: true,
                    search: None,
                }),
                None => {}
            }

            for entry in &mut group.entries {
                let index = option_rows.len();
                option_rows.push(0);

                let row = match entry {
                    Entry::Option {
                        value,
                        disabled: is_disabled,
                    } => {
                        values.push(Some(value.clone()));

                        let is_disabled = *is_disabled
                            || disabled
                                .as_ref()
                                .and_then(|disabled| disabled.get(index))
                                .copied()
                                .unwrap_or(false);

                        let (kind, search) = match view(value) {
                            Content::Text(label) => {
                                let search = label.to_lowercase();

                                (RowKind::Text(label), Some(search))
                            }
                            Content::Element(element) => (
                                RowKind::Element(element),
                                typeahead
                                    .map(|typeahead| typeahead(value))
                                    .map(|search| search.to_lowercase()),
                            ),
                        };

                        Row {
                            kind,
                            option_index: Some(index),
                            disabled: is_disabled,
                            search,
                        }
                    }
                    Entry::Deselect(content) => {
                        values.push(None);

                        let is_disabled = disabled
                            .as_ref()
                            .and_then(|disabled| disabled.get(index))
                            .copied()
                            .unwrap_or(false);

                        let (kind, search) = match content {
                            Content::Text(label) => {
                                let search = label.to_lowercase();

                                (RowKind::Text(label.clone()), Some(search))
                            }
                            Content::Element(element) => {
                                (RowKind::ElementRef(element), None)
                            }
                        };

                        Row {
                            kind,
                            option_index: Some(index),
                            disabled: is_disabled,
                            search,
                        }
                    }
                };

                rows.push(row);
            }
        }

        for (row, entry) in
            rows.iter().enumerate().filter_map(|(row_index, row)| {
                row.option_index.map(|index| (row_index, index))
            })
        {
            option_rows[entry] = row;
        }

        let aligned_row = if matches!(anchor, Anchor::Selected) {
            selected.and_then(|index| option_rows.get(index).copied())
        } else {
            None
        };

        let id = Id::unique();

        let mut list = Scrollable::new(List {
            values,
            rows,
            option_rows,
            hovered_option,
            selected,
            check_indicator,
            on_selected,
            on_option_hovered,
            width: menu_width.unwrap_or(Length::Shrink),
            spacing,
            separator,
            menu_padding,
            target_radius,
            font,
            text_size,
            line_height,
            shaping,
            ellipsis,
            padding,
            class,
        })
        .id(id.clone())
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(0)
                .margin(0)
                .scroller_width(0),
        ))
        .width(match menu_width {
            None | Some(Length::Shrink | Length::Fit) => Length::Shrink,
            _ => Length::Fill,
        })
        .height(height);

        state.tree.diff::<_, _, Renderer>(&mut list);

        let space_below = window.height - (position.y + target_height);
        let space_above = position.y;

        let max_size = Size::new(
            window.width - position.x,
            if aligned_row.is_some() {
                window.height
            } else {
                match anchor {
                    Anchor::Top => space_above,
                    Anchor::Bottom => space_below,
                    Anchor::Auto | Anchor::Selected => {
                        space_below.max(space_above)
                    }
                }
            },
        );

        let limits = match menu_width {
            None | Some(Length::Shrink | Length::Fit) => layout::Limits::new(
                Size::new(width.min(max_size.width), 0.0),
                max_size,
            ),
            Some(menu_width) => {
                layout::Limits::new(Size::ZERO, max_size).width(menu_width)
            }
        };

        list.layout(&mut state.tree, renderer, &limits);

        // when the options overflow, chevron strips at the edges take over
        // from the (hidden) scrollbar: one per direction that can scroll
        let (offset, strip_height) = state
            .tree
            .children
            .first()
            .map(|tree| {
                let state = tree.state.downcast_ref::<ListState>();

                (state.offset, state.strip_height)
            })
            .unwrap_or((0.0, 0.0));

        let content_height = state
            .tree
            .children
            .first()
            .map(|content| content.size.height)
            .unwrap_or(0.0);

        let list_height = state.tree.size.height;

        let overflows = content_height > list_height + 0.5;
        let can_scroll_up = overflows && offset > 0.5;
        let can_scroll_down =
            overflows && offset + list_height < content_height - 0.5;

        let strips = f32::from(u8::from(can_scroll_up)) * strip_height
            + f32::from(u8::from(can_scroll_down)) * strip_height;

        let list_top = if can_scroll_up { strip_height } else { 0.0 };

        if strips > 0.0 {
            let limits = layout::Limits::new(
                limits.min,
                Size::new(
                    limits.max.width,
                    (limits.max.height - strips).max(0.0),
                ),
            );

            list.layout(&mut state.tree, renderer, &limits);
        }

        let list_size = state.tree.size;
        let size = Size::new(list_size.width, list_size.height + strips);

        // a menu wider than its target centers the excess on it, clamped
        // to the window
        let x = (position.x - (size.width - width) / 2.0)
            .clamp(0.0, (window.width - size.width).max(0.0));

        let y = if let Some(row) = aligned_row {
            let row_bounds = state
                .tree
                .children
                .first()
                .and_then(|content| content.children.get(row))
                .map(|row| (row.translation.y, row.size.height));

            row_bounds
                .map(|(row_y, row_height)| {
                    position.y + (target_height - row_height) / 2.0
                        - (row_y + list_top)
                })
                .unwrap_or(position.y + target_height)
                .clamp(0.0, (window.height - size.height).max(0.0))
        } else {
            let open_below = match anchor {
                Anchor::Top => false,
                Anchor::Bottom => true,
                Anchor::Auto | Anchor::Selected => space_below > space_above,
            };

            if open_below {
                position.y + target_height
            } else {
                position.y - size.height
            }
        };

        let layout = Layout::new(size).move_to(Point::new(x, y));
        let list_layout =
            Layout::new(list_size).move_to(Point::new(x, y + list_top));

        Self {
            window,
            layout,
            list_layout,
            tree: &mut state.tree,
            list,
            id,
            target_radius,
            class,
        }
    }
}

impl<T, Message, Theme, Renderer> crate::core::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, T, Message, Theme, Renderer>
where
    T: Clone,
    Theme: Catalog + scrollable::Catalog,
    Renderer: text::Renderer,
{
    fn update(
        &mut self,
        event: &Event,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        let list_layout = self.list_layout;
        let list_bounds = list_layout.bounds();

        self.list.update(
            self.tree,
            event,
            list_layout,
            cursor,
            renderer,
            shell,
            &list_bounds,
        );

        // scroll the menu to reveal the option highlighted via keyboard
        let pending_scroll = self.tree.children.first_mut().and_then(|tree| {
            tree.state.downcast_mut::<ListState>().pending_scroll.take()
        });

        if let Some(y) = pending_scroll {
            self.list.operate(
                self.tree,
                list_layout,
                &list_bounds,
                renderer,
                &mut operation::scrollable::scroll_to::<()>(
                    self.id.clone(),
                    operation::scrollable::AbsoluteOffset {
                        x: None,
                        y: Some(y),
                    },
                    operation::Animation::Instant,
                ),
            );

            shell.request_redraw();
        }

        // hovering a chevron strip scrolls one row per tick, like the
        // scroll buttons of native menus
        let bounds = self.layout.bounds();
        let (top_strip, bottom_strip) = strips(bounds, list_bounds);

        let over_top = cursor.is_over(top_strip);
        let over_bottom = cursor.is_over(bottom_strip);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if over_top || over_bottom =>
            {
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
                if over_top || over_bottom =>
            {
                shell.request_redraw();
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                let content_height = self
                    .tree
                    .children
                    .first()
                    .map(|content| content.size.height)
                    .unwrap_or(0.0);

                let Some(tree) = self.tree.children.first_mut() else {
                    return;
                };
                let state = tree.state.downcast_mut::<ListState>();

                if !(over_top || over_bottom) {
                    state.last_auto_scroll = None;
                    return;
                }

                if state.last_auto_scroll.is_none_or(|last| {
                    now.duration_since(last) >= AUTO_SCROLL_INTERVAL
                }) {
                    state.last_auto_scroll = Some(*now);

                    let max_scroll =
                        (content_height - list_bounds.height).max(0.0);

                    let delta = if over_top {
                        -state.row_height
                    } else {
                        state.row_height
                    };
                    let target = (state.offset + delta).clamp(0.0, max_scroll);

                    self.list.operate(
                        self.tree,
                        list_layout,
                        &list_bounds,
                        renderer,
                        &mut operation::scrollable::scroll_to::<()>(
                            self.id.clone(),
                            operation::scrollable::AbsoluteOffset {
                                x: None,
                                y: Some(target),
                            },
                            operation::Animation::Instant,
                        ),
                    );
                }

                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let interaction = self.list.mouse_interaction(
            self.tree,
            self.list_layout,
            cursor,
            &Rectangle::with_size(self.window),
            renderer,
        );

        if interaction == mouse::Interaction::None
            && cursor.is_over(self.layout.bounds())
        {
            mouse::Interaction::Idle
        } else {
            interaction
        }
    }

    fn operate(
        &mut self,
        renderer: &Renderer,
        operation: &mut dyn crate::core::widget::Operation,
    ) {
        self.list.operate(
            self.tree,
            self.list_layout,
            &self.list_layout.bounds(),
            renderer,
            operation,
        );
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        cursor: mouse::Cursor,
    ) {
        let bounds = self.layout.bounds();
        let list_bounds = self.list_layout.bounds();

        let style = Catalog::style(theme, self.class);

        let border = Border {
            radius: self.target_radius.unwrap_or(style.border.radius),
            ..style.border
        };

        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border,
                    shadow: style.shadow,
                    ..renderer::Quad::default()
                },
                style.background,
            );

            self.list.draw(
                self.tree,
                renderer,
                theme,
                defaults,
                self.list_layout,
                cursor,
                &list_bounds,
            );

            let (top_strip, bottom_strip) = strips(bounds, list_bounds);

            for (strip, icon) in [
                (top_strip, Renderer::SCROLL_UP_ICON),
                (bottom_strip, Renderer::SCROLL_DOWN_ICON),
            ] {
                if strip.height <= 0.0 {
                    continue;
                }

                renderer.fill_text(
                    Text {
                        content: icon.to_string(),
                        font: Renderer::ICON_FONT,
                        size: Pixels(strip.height * 0.5),
                        line_height: renderer.line_height(),
                        bounds: strip.size(),
                        align_x: text::Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                        ellipsis: text::Ellipsis::None,
                        hint_factor: None,
                    },
                    strip.center(),
                    style.text_color,
                    bounds,
                );
            }
        });
    }
}

/// The chevron strip regions of a menu: the areas of its box not covered
/// by the scrollable list.
fn strips(bounds: Rectangle, list: Rectangle) -> (Rectangle, Rectangle) {
    (
        Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: list.y - bounds.y,
        },
        Rectangle {
            x: bounds.x,
            y: list.y + list.height,
            width: bounds.width,
            height: (bounds.y + bounds.height) - (list.y + list.height),
        },
    )
}

struct Row<'a, 'b, Theme, Renderer> {
    kind: RowKind<'a, 'b, Theme, Renderer>,
    option_index: Option<usize>,
    disabled: bool,
    search: Option<String>,
}

enum RowKind<'a, 'b, Theme, Renderer> {
    Text(String),
    Element(Element<'b, Infallible, Theme, Renderer>),
    Title(String),
    ElementRef(&'a mut Element<'b, Infallible, Theme, Renderer>),
    Divider,
}

struct List<'a, 'b, T, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    values: Vec<Option<T>>,
    rows: Vec<Row<'a, 'b, Theme, Renderer>>,
    option_rows: Vec<usize>,
    hovered_option: &'a mut Option<usize>,
    selected: Option<usize>,
    check_indicator: bool,
    on_selected: Box<dyn FnMut(Option<T>) -> Message + 'a>,
    on_option_hovered: Option<&'a dyn Fn(T) -> Message>,
    width: Length,
    spacing: Pixels,
    separator: bool,
    menu_padding: Padding,
    target_radius: Option<border::Radius>,
    padding: Padding,
    text_size: Option<Pixels>,
    line_height: Option<text::LineHeight>,
    shaping: text::Shaping,
    ellipsis: text::Ellipsis,
    font: Option<Font>,
    class: &'a <Theme as Catalog>::Class<'b>,
}

impl<T, Message, Theme, Renderer> Meta
    for List<'_, '_, T, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
}

impl<T, Message, Theme, Renderer> List<'_, '_, T, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn is_disabled(&self, index: usize) -> bool {
        self.option_rows
            .get(index)
            .is_none_or(|&row| self.rows[row].disabled)
    }

    fn value(&self, index: usize) -> Option<&T> {
        self.values.get(index).and_then(Option::as_ref)
    }

    /// The selectable option under the given cursor position, if any.
    fn option_at(
        &self,
        children: &[Tree],
        layout: Layout,
        position: Point,
    ) -> Option<usize> {
        self.rows
            .iter()
            .zip(layout.iter(children))
            .find(|(_, (row_layout, _))| row_layout.bounds().contains(position))
            .and_then(
                |(row, _)| {
                    if row.disabled { None } else { row.option_index }
                },
            )
    }
}

struct ListState {
    is_hovered: Option<bool>,
    typeahead: String,
    last_typed: Option<Instant>,
    pending_scroll: Option<f32>,
    offset: f32,
    row_height: f32,
    strip_height: f32,
    last_auto_scroll: Option<Instant>,
}

fn diff_element_row<Theme, Renderer>(
    row_tree: &mut Tree,
    element: &mut Element<'_, Infallible, Theme, Renderer>,
) where
    Renderer: crate::core::Renderer,
{
    row_tree.children.truncate(1);

    if row_tree.children.is_empty() {
        row_tree.children.push(Tree::new(&*element));
    }

    row_tree.children[0].diff(element);
}

/// The intrinsic width of an element row, when the menu shrinks to fit
/// its contents.
fn element_width<Message, Theme, Renderer>(
    widget: &mut dyn Widget<Message, Theme, Renderer>,
    tree: &mut Tree,
    renderer: &Renderer,
    max_width: f32,
) -> f32
where
    Renderer: crate::core::Renderer,
{
    let limits =
        layout::Limits::new(Size::ZERO, Size::new(max_width, f32::INFINITY));

    widget.layout(tree, renderer, &limits);

    tree.size.width
}

/// Lays out an element row: the element (the only child of `row_tree`) is
/// padded and the row takes its natural height.
fn element_row<Message, Theme, Renderer>(
    widget: &mut dyn Widget<Message, Theme, Renderer>,
    row_tree: &mut Tree,
    renderer: &Renderer,
    max_width: f32,
    padding: Padding,
    gutter: f32,
    min_height: f32,
) where
    Renderer: crate::core::Renderer,
{
    let limits = layout::Limits::new(
        Size::ZERO,
        Size::new(max_width - padding.x() - gutter, f32::INFINITY),
    );

    let child = &mut row_tree.children[0];

    widget.layout(child, renderer, &limits);

    let row_height = (child.size.height + padding.y()).max(min_height);

    child.translation = Vector::new(
        padding.left,
        padding.top + (row_height - padding.y() - child.size.height) / 2.0,
    );

    row_tree.size = Size::new(max_width, row_height);
}

impl<T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for List<'_, '_, T, Message, Theme, Renderer>
where
    T: Clone,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ListState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ListState {
            is_hovered: None,
            typeahead: String::new(),
            last_typed: None,
            pending_scroll: None,
            offset: 0.0,
            row_height: 0.0,
            strip_height: 0.0,
            last_auto_scroll: None,
        })
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children_custom(
            &mut self.rows,
            |row_tree, row| match &mut row.kind {
                RowKind::Element(element) => {
                    diff_element_row(row_tree, element);
                }
                RowKind::ElementRef(element) => {
                    diff_element_row(row_tree, element);
                }
                _ => row_tree.children.clear(),
            },
            |row| match &row.kind {
                RowKind::Element(element) => {
                    let mut row_tree = Tree::empty();
                    row_tree.children.push(Tree::new(element));
                    row_tree
                }
                RowKind::ElementRef(element) => {
                    let mut row_tree = Tree::empty();
                    row_tree.children.push(Tree::new(&**element));
                    row_tree
                }
                _ => Tree::empty(),
            },
        );
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Fit,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) {
        let text_size = self.text_size.unwrap_or_else(|| renderer.text_size());
        let line_height =
            self.line_height.unwrap_or_else(|| renderer.line_height());
        let label_text_size = Pixels(text_size.0 * LABEL_TEXT_RATIO);
        let font = self.font.unwrap_or_else(|| renderer.font());

        let option_height =
            f32::from(line_height.to_absolute(text_size)) + self.padding.y();
        let label_height = f32::from(line_height.to_absolute(label_text_size))
            + self.padding.y();

        let gutter = if self.check_indicator {
            text_size.0 * 1.5
        } else {
            0.0
        };

        let divider_height = if self.separator {
            self.spacing.0.max(1.0 + self.padding.y())
        } else {
            self.spacing.0
        };

        // rows subtract the menu padding from their horizontal padding, so
        // their text lines up with the text of the closed pick list
        let inset = Padding {
            left: (self.padding.left - self.menu_padding.left).max(0.0),
            right: (self.padding.right - self.menu_padding.right).max(0.0),
            ..self.padding
        };

        let Tree {
            state,
            children,
            size: tree_size,
            ..
        } = tree;

        {
            let state = state.downcast_mut::<ListState>();
            state.row_height = option_height;
            state.strip_height = text_size.0 + 8.0;
        }

        let bounds = limits.bounds();

        let max_width = match self.width {
            Length::Shrink | Length::Fit => {
                let shaping = self.shaping;
                let ellipsis = self.ellipsis;

                let measure = |content: &str, size: Pixels| {
                    let mut paragraph =
                        paragraph::Plain::<Renderer::Paragraph>::default();

                    let _ = paragraph.update(Text {
                        content,
                        bounds: Size::new(f32::INFINITY, option_height),
                        size,
                        line_height,
                        font,
                        align_x: text::Alignment::Default,
                        align_y: alignment::Vertical::Center,
                        shaping,
                        wrapping: text::Wrapping::None,
                        ellipsis,
                        hint_factor: renderer.hint_factor(),
                    });

                    paragraph.min_width()
                };

                let widest = self
                    .rows
                    .iter_mut()
                    .zip(children.iter_mut())
                    .fold(0.0f32, |widest, (row, row_tree)| {
                        let loose = bounds.width
                            - self.menu_padding.x()
                            - inset.x()
                            - gutter;

                        let width = match &mut row.kind {
                            RowKind::Text(label) => measure(label, text_size),
                            RowKind::Title(title) => {
                                measure(title, label_text_size)
                            }
                            RowKind::Element(element) => element_width(
                                element,
                                &mut row_tree.children[0],
                                renderer,
                                loose,
                            ),
                            RowKind::ElementRef(element) => element_width(
                                &mut **element,
                                &mut row_tree.children[0],
                                renderer,
                                loose,
                            ),
                            RowKind::Divider => 0.0,
                        };

                        widest.max(width)
                    });

                (widest + self.menu_padding.x() + inset.x() + gutter)
                    .clamp(limits.min.width, bounds.width)
            }
            _ => bounds.width,
        };

        let row_width = max_width - self.menu_padding.x();

        let mut height = self.menu_padding.top;

        for (row, row_tree) in self.rows.iter_mut().zip(children.iter_mut()) {
            let min_height = if row.option_index.is_some() {
                option_height
            } else {
                label_height
            };

            match &mut row.kind {
                RowKind::Text(_) => {
                    row_tree.size = Size::new(row_width, option_height);
                }
                RowKind::Title(_) => {
                    row_tree.size = Size::new(row_width, label_height);
                }
                RowKind::Element(element) => {
                    element_row(
                        element, row_tree, renderer, row_width, inset, gutter,
                        min_height,
                    );
                }
                RowKind::ElementRef(element) => {
                    element_row(
                        &mut **element,
                        row_tree,
                        renderer,
                        row_width,
                        inset,
                        gutter,
                        min_height,
                    );
                }
                RowKind::Divider => {
                    row_tree.size = Size::new(row_width, divider_height);
                }
            }

            row_tree.translation = Vector::new(self.menu_padding.left, height);
            height += row_tree.size.height;
        }

        *tree_size = Size::new(
            max_width,
            (height + self.menu_padding.bottom).min(bounds.height),
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(position) = cursor.position_over(layout.bounds())
                    && let Some(index) =
                        self.option_at(&tree.children, layout, position)
                {
                    *self.hovered_option = Some(index);

                    if let Some(slot) = self.values.get(index).cloned() {
                        shell.publish((self.on_selected)(slot));
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(position) = cursor.position_over(layout.bounds())
                    && let Some(index) =
                        self.option_at(&tree.children, layout, position)
                    && *self.hovered_option != Some(index)
                {
                    *self.hovered_option = Some(index);

                    if let Some(on_option_hovered) = self.on_option_hovered
                        && let Some(value) = self.value(index).cloned()
                    {
                        shell.publish(on_option_hovered(value));
                    }

                    shell.request_redraw();
                }
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                text,
                modifiers,
                ..
            }) => {
                let state = tree.state.downcast_mut::<ListState>();

                let target = match key {
                    keyboard::Key::Named(key::Named::ArrowDown) => {
                        shell.capture_event();

                        let start =
                            self.hovered_option.map_or(0, |index| index + 1);

                        (start..self.option_rows.len())
                            .find(|&index| !self.is_disabled(index))
                    }
                    keyboard::Key::Named(key::Named::ArrowUp) => {
                        shell.capture_event();

                        let end = self
                            .hovered_option
                            .unwrap_or(self.option_rows.len());

                        (0..end).rev().find(|&index| !self.is_disabled(index))
                    }
                    keyboard::Key::Named(key::Named::Home) => {
                        shell.capture_event();

                        (0..self.option_rows.len())
                            .find(|&index| !self.is_disabled(index))
                    }
                    keyboard::Key::Named(key::Named::End) => {
                        shell.capture_event();

                        (0..self.option_rows.len())
                            .rev()
                            .find(|&index| !self.is_disabled(index))
                    }
                    keyboard::Key::Named(key::Named::Enter) => {
                        shell.capture_event();

                        if let Some(index) = *self.hovered_option
                            && !self.is_disabled(index)
                            && let Some(slot) = self.values.get(index).cloned()
                        {
                            shell.publish((self.on_selected)(slot));
                        }

                        None
                    }
                    _ => {
                        let typed = text
                            .as_ref()
                            .filter(|_| {
                                !modifiers.command()
                                    && !modifiers.control()
                                    && !modifiers.alt()
                            })
                            .map(|text| {
                                text.chars()
                                    .filter(|c| !c.is_control())
                                    .collect::<String>()
                            })
                            .filter(|typed| !typed.is_empty());

                        if let Some(typed) = typed {
                            shell.capture_event();

                            let now = Instant::now();

                            if state.last_typed.is_none_or(|last| {
                                now.duration_since(last) > TYPEAHEAD_TIMEOUT
                            }) {
                                state.typeahead.clear();
                            }

                            state.typeahead.push_str(&typed.to_lowercase());
                            state.last_typed = Some(now);

                            (0..self.option_rows.len()).find(|&index| {
                                !self.is_disabled(index)
                                    && self.rows[self.option_rows[index]]
                                        .search
                                        .as_ref()
                                        .is_some_and(|search| {
                                            search.starts_with(&state.typeahead)
                                        })
                            })
                        } else {
                            None
                        }
                    }
                };

                if let Some(index) = target
                    && *self.hovered_option != Some(index)
                {
                    *self.hovered_option = Some(index);

                    if let Some(on_option_hovered) = self.on_option_hovered
                        && let Some(value) = self.value(index).cloned()
                    {
                        shell.publish(on_option_hovered(value));
                    }

                    // reveal the highlighted option if it is scrolled out
                    // of view; the overlay applies the pending scroll
                    if let Some(&row) = self.option_rows.get(index)
                        && let Some((row_layout, _)) =
                            layout.iter(&tree.children).nth(row)
                    {
                        let row_bounds = row_layout.bounds();
                        let offset = viewport.y - layout.bounds().y;

                        let top = row_bounds.y - SCROLL_MARGIN;
                        let bottom =
                            row_bounds.y + row_bounds.height + SCROLL_MARGIN;

                        if top < viewport.y {
                            state.pending_scroll =
                                Some(offset - (viewport.y - top));
                        } else if bottom > viewport.y + viewport.height {
                            state.pending_scroll = Some(
                                offset + bottom
                                    - (viewport.y + viewport.height),
                            );
                        }
                    }

                    shell.request_redraw();
                }
            }
            _ => {}
        }

        let state = tree.state.downcast_mut::<ListState>();
        state.offset = viewport.y - layout.bounds().y;

        if let Event::Window(window::Event::RedrawRequested(_now)) = event {
            state.is_hovered = Some(cursor.is_over(layout.bounds()));
        } else if state.is_hovered.is_some_and(|is_hovered| {
            is_hovered != cursor.is_over(layout.bounds())
        }) {
            shell.request_redraw();
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn crate::core::widget::Operation,
    ) {
        for (row, (row_layout, row_tree)) in self
            .rows
            .iter_mut()
            .zip(layout.iter_mut(&mut tree.children))
        {
            let bounds = row_layout.bounds();

            if !bounds.intersects(viewport) {
                continue;
            }

            match &mut row.kind {
                RowKind::Text(label) | RowKind::Title(label) => {
                    let mut text = label.clone();

                    operation.text(None, bounds, &mut text);
                }
                RowKind::Element(element) => {
                    if let Some((child_layout, child_tree)) =
                        row_layout.iter_mut(&mut row_tree.children).next()
                    {
                        element.operate(
                            child_tree,
                            child_layout,
                            viewport,
                            renderer,
                            operation,
                        );
                    }
                }
                RowKind::ElementRef(element) => {
                    if let Some((child_layout, child_tree)) =
                        row_layout.iter_mut(&mut row_tree.children).next()
                    {
                        element.operate(
                            child_tree,
                            child_layout,
                            viewport,
                            renderer,
                            operation,
                        );
                    }
                }
                RowKind::Divider => {}
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if let Some(position) = cursor.position_over(layout.bounds())
            && self.option_at(&tree.children, layout, position).is_some()
        {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let style = Catalog::style(theme, self.class);

        let text_size = self.text_size.unwrap_or_else(|| renderer.text_size());
        let line_height =
            self.line_height.unwrap_or_else(|| renderer.line_height());
        let label_text_size = Pixels(text_size.0 * LABEL_TEXT_RATIO);
        let font = self.font.unwrap_or_else(|| renderer.font());

        let gutter = if self.check_indicator {
            text_size.0 * 1.5
        } else {
            0.0
        };

        // rows subtract the menu padding from their horizontal padding, so
        // their text lines up with the text of the closed pick list
        let inset = Padding {
            left: (self.padding.left - self.menu_padding.left).max(0.0),
            right: (self.padding.right - self.menu_padding.right).max(0.0),
            ..self.padding
        };

        // the highlight radius follows the border radius of the target —
        // or of the menu, when no target radius is set — one step sharper
        let highlight_radius = {
            let radius = self.target_radius.unwrap_or(style.border.radius);

            border::Radius {
                top_left: (radius.top_left - HIGHLIGHT_RADIUS_STEP).max(0.0),
                top_right: (radius.top_right - HIGHLIGHT_RADIUS_STEP).max(0.0),
                bottom_right: (radius.bottom_right - HIGHLIGHT_RADIUS_STEP)
                    .max(0.0),
                bottom_left: (radius.bottom_left - HIGHLIGHT_RADIUS_STEP)
                    .max(0.0),
            }
        };

        let list_bounds = layout.bounds();

        for (row, (row_layout, row_tree)) in
            self.rows.iter().zip(layout.iter(&tree.children))
        {
            let bounds = row_layout.bounds();

            if !bounds.intersects(viewport) {
                continue;
            }

            let is_selectable = row.option_index.is_some();
            let is_hovered = !row.disabled
                && row
                    .option_index
                    .is_some_and(|index| *self.hovered_option == Some(index));

            if is_hovered {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        border: border::rounded(highlight_radius),
                        ..renderer::Quad::default()
                    },
                    style.selected_background,
                );
            } else if is_selectable && row.disabled {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        border: border::rounded(highlight_radius),
                        ..renderer::Quad::default()
                    },
                    style.disabled_background,
                );
            }

            let text_color = if is_selectable && row.disabled {
                style.disabled_text_color
            } else if is_hovered {
                style.selected_text_color
            } else {
                style.text_color
            };

            if self.check_indicator
                && row
                    .option_index
                    .is_some_and(|index| self.selected == Some(index))
            {
                let check_size = Pixels(text_size.0 * 0.75);

                renderer.fill_text(
                    Text {
                        content: Renderer::CHECKMARK_ICON.to_string(),
                        font: Renderer::ICON_FONT,
                        size: check_size,
                        line_height,
                        bounds: Size::new(
                            bounds.width,
                            f32::from(line_height.to_absolute(check_size)),
                        ),
                        align_x: text::Alignment::Right,
                        align_y: alignment::Vertical::Center,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                        ellipsis: text::Ellipsis::None,
                        hint_factor: None,
                    },
                    Point::new(
                        bounds.x + bounds.width - inset.right,
                        bounds.center_y(),
                    ),
                    text_color,
                    *viewport,
                );
            }

            match &row.kind {
                RowKind::Text(label) => {
                    renderer.fill_text(
                        Text {
                            content: label.clone(),
                            bounds: Size::new(
                                bounds.width - inset.x() - gutter,
                                bounds.height,
                            ),
                            size: text_size,
                            line_height,
                            font,
                            align_x: text::Alignment::Default,
                            align_y: alignment::Vertical::Center,
                            shaping: self.shaping,
                            wrapping: text::Wrapping::None,
                            ellipsis: self.ellipsis,
                            hint_factor: renderer.hint_factor(),
                        },
                        Point::new(bounds.x + inset.left, bounds.center_y()),
                        text_color,
                        *viewport,
                    );
                }
                RowKind::Title(title) => {
                    renderer.fill_text(
                        Text {
                            content: title.clone(),
                            bounds: Size::new(
                                bounds.width - inset.x(),
                                bounds.height,
                            ),
                            size: label_text_size,
                            line_height,
                            font,
                            align_x: text::Alignment::Default,
                            align_y: alignment::Vertical::Center,
                            shaping: self.shaping,
                            wrapping: text::Wrapping::None,
                            ellipsis: self.ellipsis,
                            hint_factor: renderer.hint_factor(),
                        },
                        Point::new(bounds.x + inset.left, bounds.center_y()),
                        style.label_text_color,
                        *viewport,
                    );
                }
                RowKind::Element(element) => {
                    if let Some((child_layout, child_tree)) =
                        row_layout.iter(&row_tree.children).next()
                    {
                        element.draw(
                            child_tree,
                            renderer,
                            theme,
                            &renderer::Style { text_color },
                            child_layout,
                            cursor,
                            viewport,
                        );
                    }
                }
                RowKind::ElementRef(element) => {
                    if let Some((child_layout, child_tree)) =
                        row_layout.iter(&row_tree.children).next()
                    {
                        element.draw(
                            child_tree,
                            renderer,
                            theme,
                            &renderer::Style {
                                text_color: if is_selectable {
                                    text_color
                                } else {
                                    style.label_text_color
                                },
                            },
                            child_layout,
                            cursor,
                            viewport,
                        );
                    }
                }
                RowKind::Divider => {
                    if self.separator {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: list_bounds.x,
                                    y: bounds.center_y() - 0.5,
                                    width: list_bounds.width,
                                    height: 1.0,
                                },
                                ..renderer::Quad::default()
                            },
                            style.separator_color,
                        );
                    }
                }
            }
        }
    }
}

/// The appearance of a [`Menu`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the menu.
    pub background: Background,
    /// The [`Border`] of the menu.
    pub border: Border,
    /// The text [`Color`] of the menu.
    pub text_color: Color,
    /// The text [`Color`] of a selected option in the menu.
    pub selected_text_color: Color,
    /// The background [`Color`] of a selected option in the menu.
    pub selected_background: Background,
    /// The [`Shadow`] of the menu.
    pub shadow: Shadow,
    /// The text [`Color`] of a disabled option in the menu.
    pub disabled_text_color: Color,
    /// The background [`Color`] of a disabled option in the menu.
    pub disabled_background: Background,
    /// The text [`Color`] of a group label in the menu.
    pub label_text_color: Color,
    /// The [`Color`] of the rule separating groups in the menu.
    pub separator_color: Color,
}

/// The theme catalog of a [`Menu`].
pub trait Catalog: scrollable::Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> <Self as Catalog>::Class<'a>;

    /// The default class for the scrollable of the [`Menu`].
    fn default_scrollable<'a>() -> <Self as scrollable::Catalog>::Class<'a> {
        <Self as scrollable::Catalog>::default()
    }

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &<Self as Catalog>::Class<'_>) -> Style;
}

/// A styling function for a [`Menu`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default)
    }

    fn style(&self, class: &StyleFn<'_, Self>) -> Style {
        class(self)
    }
}

/// The classic iced style of the list of a [`Menu`]: the hovered option
/// takes the primary color.
pub fn legacy(theme: &Theme) -> Style {
    let palette = theme.palette();

    Style {
        selected_text_color: palette.primary.strong.text,
        selected_background: palette.primary.strong.color.into(),
        ..default(theme)
    }
}

/// The default style of the list of a [`Menu`].
///
/// The hovered option gets a neutral wash; selection is communicated by
/// the check indicator. For the classic iced styling, see [`legacy`].
pub fn default(theme: &Theme) -> Style {
    let palette = theme.palette();

    Style {
        background: palette.background.weak.color.into(),
        border: Border {
            width: 1.0,
            radius: 0.0.into(),
            color: palette.background.strong.color,
        },
        text_color: palette.background.weak.text,
        selected_text_color: palette.background.strong.text,
        selected_background: palette.background.strong.color.into(),
        shadow: Shadow::default(),
        disabled_text_color: palette.background.strong.color,
        disabled_background: palette.background.weak.color.into(),
        label_text_color: palette.secondary.base.color,
        separator_color: palette.background.strong.color,
    }
}
