//! Content wrapper for tile grid items.
//!
//! [`Content`] wraps the body [`Element`] of a grid item with an optional
//! [`TitleBar`] and container styling. It handles layout (title bar on top,
//! body below), drawing, event propagation, and overlays.

use iced_widget::container;

use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::{self, Tree};
use crate::core::{
    self, Background, Border, Color, Element, Event, Layout, Point, Rectangle,
    Shell, Size, Vector,
};

use super::TitleBar;

/// The content of an item in a [`TileGrid`].
///
/// Wraps a body element with an optional [`TitleBar`] and container styling.
/// The title bar (if present) is drawn at the top of the allocated space,
/// and the body fills the remaining area below it.
///
/// # Example
///
/// ```ignore
/// use sweeten::widget::tile_grid::{Content, TitleBar};
/// use iced::widget::text;
///
/// let content = Content::new(text("Hello!"))
///     .title_bar(TitleBar::new(text("My Item")).padding(5));
/// ```
///
/// [`TileGrid`]: super::TileGrid
pub struct Content<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    title_bar: Option<TitleBar<'a, Message, Theme, Renderer>>,
    body: Element<'a, Message, Theme, Renderer>,
    class: Theme::Class<'a>,
    draggable: bool,
    resizable: bool,
}

impl<'a, Message, Theme, Renderer> Content<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    /// Creates a new [`Content`] with the provided body.
    ///
    /// By default, the content is both draggable and resizable.
    pub fn new(body: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            title_bar: None,
            body: body.into(),
            class: Theme::default(),
            draggable: true,
            resizable: true,
        }
    }

    /// Sets the [`TitleBar`] of the [`Content`].
    pub fn title_bar(
        mut self,
        title_bar: TitleBar<'a, Message, Theme, Renderer>,
    ) -> Self {
        self.title_bar = Some(title_bar);
        self
    }

    /// Sets the style of the [`Content`].
    #[must_use]
    pub fn style(
        mut self,
        style: impl Fn(&Theme) -> container::Style + 'a,
    ) -> Self
    where
        Theme::Class<'a>: From<container::StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as container::StyleFn<'a, Theme>).into();
        self
    }

    /// Sets whether this item can be dragged to move it.
    ///
    /// When `false`, the widget will not initiate a drag interaction
    /// on this item, even if the cursor is over its title bar.
    /// Defaults to `true`.
    #[must_use]
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Sets whether this item can be resized by dragging its edges.
    ///
    /// When `false`, the widget will not show a resize grip or allow
    /// resize interactions on this item. Defaults to `true`.
    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

impl<Message, Theme, Renderer> Content<'_, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    /// Returns whether this content is draggable.
    pub(crate) fn is_draggable(&self) -> bool {
        self.draggable
    }

    /// Returns whether this content is resizable.
    pub(crate) fn is_resizable(&self) -> bool {
        self.resizable
    }

    pub(super) fn state(&self) -> Tree {
        let children = if let Some(title_bar) = self.title_bar.as_ref() {
            vec![Tree::new(&self.body), title_bar.state()]
        } else {
            vec![Tree::new(&self.body), Tree::empty()]
        };

        Tree {
            children,
            ..Tree::empty()
        }
    }

    pub(super) fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 2 {
            if let Some(title_bar) = self.title_bar.as_ref() {
                title_bar.diff(&mut tree.children[1]);
            }
            tree.children[0].diff(&self.body);
        } else {
            *tree = self.state();
        }
    }

    /// Draws the [`Content`] with the provided [`Renderer`] and [`Layout`].
    ///
    /// The draw is split into three passes:
    /// 1. Background fill (no border) — drawn first
    /// 2. Title bar + body — drawn on top of the fill
    /// 3. Border overlay — drawn last, so it is never obscured by the title bar
    ///
    /// [`Renderer`]: core::Renderer
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let content_style = container::Catalog::style(theme, &self.class);

        // Pass 1: draw background fill only (no border stroke, but WITH radius for clipping).
        {
            let fill_style = container::Style {
                border: Border {
                    width: 0.0,
                    color: Color::TRANSPARENT,
                    radius: content_style.border.radius,
                },
                ..content_style
            };
            container::draw_background(renderer, &fill_style, bounds);
        }

        // Pass 2: draw children (title bar + body).
        if let Some(title_bar) = &self.title_bar {
            let mut children = layout.children();
            let title_bar_layout = children.next().unwrap();
            let body_layout = children.next().unwrap();

            let show_controls = cursor.is_over(bounds);

            self.body.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                body_layout,
                cursor,
                viewport,
            );

            title_bar.draw(
                &tree.children[1],
                renderer,
                theme,
                style,
                title_bar_layout,
                cursor,
                viewport,
                show_controls,
                content_style.border.radius,
            );
        } else {
            self.body.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        }

        // Pass 3: draw border overlay on top of everything.
        if content_style.border.width > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: content_style.border,
                    ..renderer::Quad::default()
                },
                Background::Color(Color::TRANSPARENT),
            );
        }
    }

    pub(crate) fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if let Some(title_bar) = &mut self.title_bar {
            let max_size = limits.max();

            let title_bar_layout = title_bar.layout(
                &mut tree.children[1],
                renderer,
                &layout::Limits::new(Size::ZERO, max_size),
            );

            let title_bar_size = title_bar_layout.size();

            let body_layout = self.body.as_widget_mut().layout(
                &mut tree.children[0],
                renderer,
                &layout::Limits::new(
                    Size::ZERO,
                    Size::new(
                        max_size.width,
                        max_size.height - title_bar_size.height,
                    ),
                ),
            );

            layout::Node::with_children(
                max_size,
                vec![
                    title_bar_layout,
                    body_layout.move_to(Point::new(0.0, title_bar_size.height)),
                ],
            )
        } else {
            self.body.as_widget_mut().layout(
                &mut tree.children[0],
                renderer,
                limits,
            )
        }
    }

    pub(crate) fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let body_layout = if let Some(title_bar) = &mut self.title_bar {
            let mut children = layout.children();

            title_bar.operate(
                &mut tree.children[1],
                children.next().unwrap(),
                renderer,
                operation,
            );

            children.next().unwrap()
        } else {
            layout
        };

        self.body.as_widget_mut().operate(
            &mut tree.children[0],
            body_layout,
            renderer,
            operation,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
        is_picked: bool,
    ) {
        let body_layout = if let Some(title_bar) = &mut self.title_bar {
            let mut children = layout.children();

            title_bar.update(
                &mut tree.children[1],
                event,
                children.next().unwrap(),
                cursor,
                renderer,
                shell,
                viewport,
            );

            children.next().unwrap()
        } else {
            layout
        };

        if !is_picked {
            self.body.as_widget_mut().update(
                &mut tree.children[0],
                event,
                body_layout,
                cursor,
                renderer,
                shell,
                viewport,
            );
        }
    }

    pub(crate) fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
        drag_enabled: bool,
    ) -> mouse::Interaction {
        let (body_layout, title_bar_interaction) = if let Some(title_bar) =
            &self.title_bar
        {
            let mut children = layout.children();
            let title_bar_layout = children.next().unwrap();

            let is_over_pick_area = cursor
                .position()
                .map(|cursor_position| {
                    title_bar
                        .is_over_pick_area(title_bar_layout, cursor_position)
                })
                .unwrap_or_default();

            if is_over_pick_area && drag_enabled {
                return mouse::Interaction::Grab;
            }

            let mouse_interaction = title_bar.mouse_interaction(
                &tree.children[1],
                title_bar_layout,
                cursor,
                viewport,
                renderer,
            );

            (children.next().unwrap(), mouse_interaction)
        } else {
            (layout, mouse::Interaction::default())
        };

        self.body
            .as_widget()
            .mouse_interaction(
                &tree.children[0],
                body_layout,
                cursor,
                viewport,
                renderer,
            )
            .max(title_bar_interaction)
    }

    /// Returns the mouse interaction when hovering over the pick area of the
    /// title bar, if any. Used by the parent grid for redraw detection.
    pub(crate) fn grid_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        drag_enabled: bool,
    ) -> Option<mouse::Interaction> {
        let title_bar = self.title_bar.as_ref()?;
        let mut children = layout.children();
        let title_bar_layout = children.next()?;

        let is_over_pick_area = cursor
            .position()
            .map(|cursor_position| {
                title_bar.is_over_pick_area(title_bar_layout, cursor_position)
            })
            .unwrap_or_default();

        if is_over_pick_area && drag_enabled {
            return Some(mouse::Interaction::Grab);
        }

        None
    }

    /// Returns whether the cursor is over the title bar pick area (for drag detection).
    pub(crate) fn can_be_dragged_at(
        &self,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> bool {
        if let Some(title_bar) = &self.title_bar {
            let mut children = layout.children();
            let title_bar_layout = children.next().unwrap();
            title_bar.is_over_pick_area(title_bar_layout, cursor_position)
        } else {
            false
        }
    }

    pub(crate) fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        if let Some(title_bar) = self.title_bar.as_mut() {
            let mut children = layout.children();
            let title_bar_layout = children.next()?;

            let mut states = tree.children.iter_mut();
            let body_state = states.next().unwrap();
            let title_bar_state = states.next().unwrap();

            match title_bar.overlay(
                title_bar_state,
                title_bar_layout,
                renderer,
                viewport,
                translation,
            ) {
                Some(overlay) => Some(overlay),
                None => self.body.as_widget_mut().overlay(
                    body_state,
                    children.next()?,
                    renderer,
                    viewport,
                    translation,
                ),
            }
        } else {
            self.body.as_widget_mut().overlay(
                &mut tree.children[0],
                layout,
                renderer,
                viewport,
                translation,
            )
        }
    }
}

impl<'a, T, Message, Theme, Renderer> From<T>
    for Content<'a, Message, Theme, Renderer>
where
    T: Into<Element<'a, Message, Theme, Renderer>>,
    Theme: container::Catalog + 'a,
    Renderer: core::Renderer,
{
    fn from(element: T) -> Self {
        Self::new(element)
    }
}
