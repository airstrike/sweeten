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
    Vector, Widget,
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
    pub fn new(content: impl Widget<Message, Theme, Renderer> + 'a) -> Self {
        Self {
            content: content._boxed(),
            controls: None,
            padding: Padding::ZERO,
            always_show_controls: false,
            class: Theme::default(),
        }
    }

    /// Sets the controls of the [`TitleBar`].
    pub fn controls(
        mut self,
        controls: impl Widget<Message, Theme, Renderer> + 'a,
    ) -> Self {
        self.controls = Some(controls._boxed());
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

    pub(super) fn diff(&mut self, tree: &mut Tree) {
        if tree.children.len() != 2 {
            *tree = self.state();
        }

        if let Some(controls) = self.controls.as_mut() {
            tree.children[1].diff(controls);
        }
        tree.children[0].diff(&mut self.content);
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
        layout: Layout,
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

        let mut children = layout.iter(&tree.children);
        let (title_layout, title_tree) = children.next().unwrap();

        if let Some(controls) = &self.controls
            && (show_controls || self.always_show_controls)
        {
            let (controls_layout, controls_tree) = children.next().unwrap();

            controls.draw(
                controls_tree,
                renderer,
                theme,
                &inherited_style,
                controls_layout,
                cursor,
                viewport,
            );
        }

        self.content.draw(
            title_tree,
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
        tree: &Tree,
        layout: Layout,
        cursor_position: Point,
    ) -> bool {
        if layout.bounds().contains(cursor_position) {
            let mut children = layout.iter(&tree.children);
            let (title_layout, _) = children.next().unwrap();

            if self.controls.is_some() {
                let (controls_layout, _) = children.next().unwrap();
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
    ) {
        let limits = limits.shrink(self.padding);
        let max_size = limits.max;

        self.content.layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, max_size),
        );

        let title_size = tree.children[0].size;
        let padding = Vector::new(self.padding.left, self.padding.top);

        tree.children[0].translation = padding;

        let size = if let Some(controls) = &mut self.controls {
            controls.layout(
                &mut tree.children[1],
                renderer,
                &layout::Limits::new(Size::ZERO, max_size),
            );

            let controls_size = tree.children[1].size;
            let space_before_controls = max_size.width - controls_size.width;
            let height = title_size.height.max(controls_size.height);

            tree.children[1].translation =
                padding + Vector::new(space_before_controls, 0.0);

            Size::new(max_size.width, height)
        } else {
            Size::new(max_size.width, title_size.height)
        };

        tree.size = size.expand(self.padding);
    }

    pub(crate) fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let mut children = layout.iter_mut(&mut tree.children);
        let (title_layout, title_tree) = children.next().unwrap();

        if let Some(controls) = &mut self.controls {
            let (controls_layout, controls_tree) = children.next().unwrap();
            controls.operate(
                controls_tree,
                controls_layout,
                viewport,
                renderer,
                operation,
            );
        }

        self.content.operate(
            title_tree,
            title_layout,
            viewport,
            renderer,
            operation,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut children = layout.iter_mut(&mut tree.children);
        let (title_layout, title_tree) = children.next().unwrap();

        if let Some(controls) = &mut self.controls {
            let (controls_layout, controls_tree) = children.next().unwrap();
            controls.update(
                controls_tree,
                event,
                controls_layout,
                cursor,
                renderer,
                shell,
                viewport,
            );
        }

        self.content.update(
            title_tree,
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
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut children = layout.iter(&tree.children);
        let (title_layout, title_tree) = children.next().unwrap();

        let title_interaction = self.content.mouse_interaction(
            title_tree,
            title_layout,
            cursor,
            viewport,
            renderer,
        );

        if let Some(controls) = &self.controls {
            let (controls_layout, controls_tree) = children.next().unwrap();
            let controls_interaction = controls.mouse_interaction(
                controls_tree,
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
        layout: Layout,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = layout.iter_mut(&mut tree.children);
        let Some((title_layout, title_state)) = children.next() else {
            return Vec::new();
        };

        let Self {
            content, controls, ..
        } = self;

        let overlays = content.overlay(
            title_state,
            title_layout,
            renderer,
            viewport,
            translation,
            window,
        );

        if !overlays.is_empty() {
            return overlays;
        }

        match (controls.as_mut(), children.next()) {
            (Some(controls), Some((controls_layout, controls_state))) => {
                controls.overlay(
                    controls_state,
                    controls_layout,
                    renderer,
                    viewport,
                    translation,
                    window,
                )
            }
            (None, _) | (_, None) => Vec::new(),
        }
    }
}
