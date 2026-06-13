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
    controls: Option<Element<'a, Message, Theme, Renderer>>,
    class: Theme::Class<'a>,
    draggable: bool,
    drag_body: bool,
    resizable: bool,
    held: bool,
    hug_height: bool,
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
            controls: None,
            class: Theme::default(),
            draggable: true,
            drag_body: false,
            resizable: true,
            held: false,
            hug_height: false,
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

    /// Sets a controls element pinned to the item's top-right corner.
    ///
    /// Unlike [`TitleBar::controls`], which lay out *inside* the title bar,
    /// these are rendered as an **overlay** — like a [`pick_list`] menu — so
    /// they can straddle the item's top edge (the card's top border passes
    /// through their vertical center) and are never clipped by the item.
    ///
    /// [`TitleBar::controls`]: super::TitleBar::controls
    /// [`pick_list`]: crate::widget::pick_list
    pub fn controls(
        mut self,
        controls: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.controls = Some(controls.into());
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

    /// Sets whether the whole card acts as a drag handle, not just the title
    /// bar.
    ///
    /// By default a drag only starts from the title bar's pick area (the strip
    /// minus its title content and controls), matching [`pane_grid`]. Enable
    /// this for dashboard-style tiles where grabbing anywhere on the card to
    /// move it feels more natural. Has no effect unless [`draggable`] is also
    /// set (the default). The resize grip and the controls overlay still take
    /// priority over their own regions.
    ///
    /// [`draggable`]: Self::draggable
    /// [`pane_grid`]: https://docs.iced.rs/iced/widget/pane_grid
    #[must_use]
    pub fn drag_body(mut self, drag_body: bool) -> Self {
        self.drag_body = drag_body;
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

    /// Marks this item as held — it will not be displaced by drags
    /// on other items.
    ///
    /// Use this to express per-item "pinned" or "sticky" intent
    /// derived from your domain data (e.g. `item.is_pinned`).
    /// Defaults to `false`.
    #[must_use]
    pub fn held(mut self, held: bool) -> Self {
        self.held = held;
        self
    }

    /// Shrinks the painted content height to the title bar plus the
    /// body's natural height, capped by the grid cell allocation.
    ///
    /// The parent grid coordinates remain fixed; this only changes the
    /// visual/content bounds inside the allocated item slot. It is
    /// useful for bottom-anchored table/report tiles whose authored
    /// grid span is an upper bound rather than a desired card height.
    #[must_use]
    pub fn hug_height(mut self) -> Self {
        self.hug_height = true;
        self
    }

    /// Renders this [`Content`]'s body as a standalone styled [`Element`],
    /// outside any [`TileGrid`].
    ///
    /// The container styling set via [`style`](Self::style) is applied; the
    /// body fills the element. Drag/resize/title-bar/controls are dropped —
    /// they only have meaning inside the grid widget. Use this to render an
    /// item's card in a separate, non-dragged region (e.g. a fixed sidebar
    /// column) while keeping the same per-tile view code.
    ///
    /// [`TileGrid`]: super::TileGrid
    #[must_use]
    pub fn into_panel(self) -> Element<'a, Message, Theme, Renderer>
    where
        Message: 'a,
        Theme: 'a,
        Renderer: 'a,
    {
        container(self.body)
            .width(core::Length::Fill)
            .height(core::Length::Fill)
            .class(self.class)
            .into()
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

    /// Returns whether the whole card (not just the title bar) is a drag
    /// handle. For a container this also makes its body a drag handle.
    pub(crate) fn drags_from_body(&self) -> bool {
        self.drag_body
    }

    /// Returns whether this content is resizable.
    pub(crate) fn is_resizable(&self) -> bool {
        self.resizable
    }

    /// Returns whether this content is held (not displaceable).
    pub(crate) fn is_held(&self) -> bool {
        self.held
    }

    /// Returns whether this content has a title bar.
    pub(crate) fn has_title_bar(&self) -> bool {
        self.title_bar.is_some()
    }

    pub(super) fn state(&self) -> Tree {
        let title_bar_state = self
            .title_bar
            .as_ref()
            .map_or_else(Tree::empty, TitleBar::state);
        let controls_state = self
            .controls
            .as_ref()
            .map_or_else(Tree::empty, |controls| Tree::new(controls));

        Tree {
            children: vec![
                Tree::new(&self.body),
                title_bar_state,
                controls_state,
            ],
            ..Tree::empty()
        }
    }

    pub(super) fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 3 {
            if let Some(title_bar) = self.title_bar.as_ref() {
                title_bar.diff(&mut tree.children[1]);
            }
            if let Some(controls) = self.controls.as_ref() {
                tree.children[2].diff(controls);
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
            let body_max_height =
                (max_size.height - title_bar_size.height).max(0.0);

            let body_layout = self.body.as_widget_mut().layout(
                &mut tree.children[0],
                renderer,
                &layout::Limits::new(
                    Size::ZERO,
                    Size::new(max_size.width, body_max_height),
                ),
            );

            let height = if self.hug_height {
                (title_bar_size.height + body_layout.size().height)
                    .min(max_size.height)
            } else {
                max_size.height
            };

            layout::Node::with_children(
                Size::new(max_size.width, height),
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
        // The whole card is a drag handle when opted in (the controls overlay
        // and resize grip, checked earlier, still claim their own regions).
        if self.drag_body {
            return layout.bounds().contains(cursor_position);
        }
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
        show_controls: bool,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let bounds = layout.bounds();

        let mut states = tree.children.iter_mut();
        let body_state = states.next().unwrap();
        let title_bar_state = states.next().unwrap();
        let controls_state = states.next().unwrap();

        let mut group: Vec<overlay::Element<'b, Message, Theme, Renderer>> =
            Vec::new();

        // Overlays bubbling up from the title bar or body (e.g. a menu).
        let inner = if let Some(title_bar) = self.title_bar.as_mut() {
            let mut children = layout.children();
            let title_bar_layout = children.next();
            let body_layout = children.next();

            title_bar_layout.and_then(|title_bar_layout| {
                match title_bar.overlay(
                    title_bar_state,
                    title_bar_layout,
                    renderer,
                    viewport,
                    translation,
                ) {
                    Some(overlay) => Some(overlay),
                    None => body_layout.and_then(|body_layout| {
                        self.body.as_widget_mut().overlay(
                            body_state,
                            body_layout,
                            renderer,
                            viewport,
                            translation,
                        )
                    }),
                }
            })
        } else {
            self.body.as_widget_mut().overlay(
                body_state,
                layout,
                renderer,
                viewport,
                translation,
            )
        };
        if let Some(inner) = inner {
            group.push(inner);
        }

        // Controls overlay, anchored straddling the item's top edge so the
        // card's top border passes through its vertical center. Shown only
        // while the item is hovered.
        if show_controls && let Some(controls) = self.controls.as_mut() {
            let anchor = Rectangle {
                x: bounds.x + translation.x,
                y: bounds.y + translation.y,
                ..bounds
            };
            group.push(overlay::Element::new(Box::new(ControlsOverlay {
                element: controls,
                tree: controls_state,
                anchor,
            })));
        }

        if group.is_empty() {
            None
        } else {
            Some(overlay::Group::with_children(group).overlay())
        }
    }
}

/// An [`overlay`] that renders a [`Content`]'s controls element anchored to
/// the top-right of the item, lifted so the item's top edge bisects it.
struct ControlsOverlay<'a, 'b, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
{
    element: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut Tree,
    anchor: Rectangle,
}

impl<Message, Theme, Renderer> core::Overlay<Message, Theme, Renderer>
    for ControlsOverlay<'_, '_, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.element.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let size = node.size();

        const MARGIN: f32 = 6.0;
        let x =
            (self.anchor.x + self.anchor.width - size.width - MARGIN).max(0.0);
        let y = (self.anchor.y - size.height / 2.0).max(0.0);

        node.move_to(Point::new(x, y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();
        self.element
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, shell, &bounds);
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();
        self.element.as_widget().draw(
            &*self.tree,
            renderer,
            theme,
            defaults,
            layout,
            cursor,
            &bounds,
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        self.element.as_widget().mouse_interaction(
            &*self.tree,
            layout,
            cursor,
            &bounds,
            renderer,
        )
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
