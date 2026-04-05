//! Title bar for tile grid items.
//!
//! A [`TitleBar`] sits at the top of a [`Content`] and provides a draggable
//! area plus optional controls (close button, pin toggle, etc.).
//!
//! [`Content`]: super::Content

use iced_widget::container;

use crate::core::border;
use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::{self, Tree};
use crate::core::{
    self, Element, Event, Layout, Padding, Point, Rectangle, Shell, Size,
    Vector,
};

/// The title bar of an item in a [`TileGrid`].
///
/// The title bar is placed at the top of a [`Content`] and serves as the
/// drag handle for moving items. Controls (buttons, toggles, etc.) can be
/// placed on the right side of the title bar.
///
/// The *pick area* — the region where dragging is initiated — covers the
/// entire title bar **except** the controls and the title content element.
///
/// [`TileGrid`]: super::TileGrid
/// [`Content`]: super::Content
pub struct TitleBar<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    controls: Option<Element<'a, Message, Theme, Renderer>>,
    padding: Padding,
    always_show_controls: bool,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme, Renderer> TitleBar<'a, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    /// Creates a new [`TitleBar`] with the given content.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            content: content.into(),
            controls: None,
            padding: Padding::ZERO,
            always_show_controls: false,
            class: Theme::default(),
        }
    }

    /// Sets the controls of the [`TitleBar`].
    pub fn controls(
        mut self,
        controls: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.controls = Some(controls.into());
        self
    }

    /// Sets the [`Padding`] of the [`TitleBar`].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets whether the controls are always visible.
    ///
    /// By default, controls are only visible when the parent item is hovered.
    pub fn always_show_controls(mut self) -> Self {
        self.always_show_controls = true;
        self
    }

    /// Sets the style of the [`TitleBar`].
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
}

impl<Message, Theme, Renderer> TitleBar<'_, Message, Theme, Renderer>
where
    Theme: container::Catalog,
    Renderer: core::Renderer,
{
    pub(super) fn state(&self) -> Tree {
        let children = if let Some(controls) = self.controls.as_ref() {
            vec![Tree::new(&self.content), Tree::new(controls)]
        } else {
            vec![Tree::new(&self.content), Tree::empty()]
        };

        Tree {
            children,
            ..Tree::empty()
        }
    }

    pub(super) fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 2 {
            if let Some(controls) = self.controls.as_ref() {
                tree.children[1].diff(controls);
            }
            tree.children[0].diff(&self.content);
        } else {
            *tree = self.state();
        }
    }

    /// Draws the [`TitleBar`].
    ///
    /// `parent_radius` is the border radius of the parent [`Content`], used to
    /// round the top corners of the title bar background so it matches the card.
    ///
    /// [`Content`]: super::Content
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        inherited_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        show_controls: bool,
        parent_radius: border::Radius,
    ) {
        let bounds = layout.bounds();
        let mut style = container::Catalog::style(theme, &self.class);

        // Apply the parent card's top corner radii so the title bar
        // background does not poke outside the rounded card.
        style.border.radius = border::Radius {
            top_left: parent_radius.top_left,
            top_right: parent_radius.top_right,
            bottom_right: 0.0,
            bottom_left: 0.0,
        };

        let inherited_style = renderer::Style {
            text_color: style.text_color.unwrap_or(inherited_style.text_color),
        };

        container::draw_background(renderer, &style, bounds);

        let mut children = layout.children();
        let padded = children.next().unwrap();
        let mut children = padded.children();
        let title_layout = children.next().unwrap();

        if let Some(controls) = &self.controls
            && (show_controls || self.always_show_controls)
        {
            let controls_layout = children.next().unwrap();

            controls.as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                &inherited_style,
                controls_layout,
                cursor,
                viewport,
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &inherited_style,
            title_layout,
            cursor,
            viewport,
        );
    }

    /// Returns whether the cursor is over the pick area (drag handle).
    ///
    /// The pick area is the title bar excluding the controls and title content.
    pub fn is_over_pick_area(
        &self,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> bool {
        if layout.bounds().contains(cursor_position) {
            let mut children = layout.children();
            let padded = children.next().unwrap();
            let mut children = padded.children();
            let title_layout = children.next().unwrap();

            if self.controls.is_some() {
                let controls_layout = children.next().unwrap();
                !controls_layout.bounds().contains(cursor_position)
                    && !title_layout.bounds().contains(cursor_position)
            } else {
                !title_layout.bounds().contains(cursor_position)
            }
        } else {
            false
        }
    }

    pub(crate) fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.shrink(self.padding);
        let max_size = limits.max();

        let title_layout = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, max_size),
        );

        let title_size = title_layout.size();

        let node = if let Some(controls) = &mut self.controls {
            let controls_layout = controls.as_widget_mut().layout(
                &mut tree.children[1],
                renderer,
                &layout::Limits::new(Size::ZERO, max_size),
            );

            let controls_size = controls_layout.size();
            let space_before_controls = max_size.width - controls_size.width;
            let height = title_size.height.max(controls_size.height);

            layout::Node::with_children(
                Size::new(max_size.width, height),
                vec![
                    title_layout,
                    controls_layout
                        .move_to(Point::new(space_before_controls, 0.0)),
                ],
            )
        } else {
            layout::Node::with_children(
                Size::new(max_size.width, title_size.height),
                vec![title_layout],
            )
        };

        layout::Node::container(node, self.padding)
    }

    pub(crate) fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let mut children = layout.children();
        let padded = children.next().unwrap();
        let mut children = padded.children();
        let title_layout = children.next().unwrap();

        if let Some(controls) = &mut self.controls {
            let controls_layout = children.next().unwrap();
            controls.as_widget_mut().operate(
                &mut tree.children[1],
                controls_layout,
                renderer,
                operation,
            );
        }

        self.content.as_widget_mut().operate(
            &mut tree.children[0],
            title_layout,
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
    ) {
        let mut children = layout.children();
        let padded = children.next().unwrap();
        let mut children = padded.children();
        let title_layout = children.next().unwrap();

        if let Some(controls) = &mut self.controls {
            let controls_layout = children.next().unwrap();
            controls.as_widget_mut().update(
                &mut tree.children[1],
                event,
                controls_layout,
                cursor,
                renderer,
                shell,
                viewport,
            );
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            title_layout,
            cursor,
            renderer,
            shell,
            viewport,
        );
    }

    pub(crate) fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut children = layout.children();
        let padded = children.next().unwrap();
        let mut children = padded.children();
        let title_layout = children.next().unwrap();

        let title_interaction = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            title_layout,
            cursor,
            viewport,
            renderer,
        );

        if let Some(controls) = &self.controls {
            let controls_layout = children.next().unwrap();
            let controls_interaction = controls.as_widget().mouse_interaction(
                &tree.children[1],
                controls_layout,
                cursor,
                viewport,
                renderer,
            );
            controls_interaction.max(title_interaction)
        } else {
            title_interaction
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
        let mut children = layout.children();
        let padded = children.next()?;
        let mut children = padded.children();
        let title_layout = children.next()?;

        let Self {
            content, controls, ..
        } = self;

        let mut states = tree.children.iter_mut();
        let title_state = states.next().unwrap();
        let controls_state = states.next().unwrap();

        content
            .as_widget_mut()
            .overlay(title_state, title_layout, renderer, viewport, translation)
            .or_else(move || {
                controls.as_mut().and_then(|controls| {
                    let controls_layout = children.next()?;
                    controls.as_widget_mut().overlay(
                        controls_state,
                        controls_layout,
                        renderer,
                        viewport,
                        translation,
                    )
                })
            })
    }
}
