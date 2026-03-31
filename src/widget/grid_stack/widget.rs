//! The [`GridStack`] widget implementation.
//!
//! This module contains the main [`GridStack`] widget, event types, and the
//! [`Catalog`] trait for theming.

use std::collections::HashMap;

use iced_widget::container;

use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay::{self, Group};
use crate::core::renderer;
use crate::core::time::Instant;
use crate::core::touch;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    self, Animation, Background, Border, Color, Element, Event, Layout, Length,
    Pixels, Point, Rectangle, Shell, Size, Theme, Vector, Widget,
};

use super::content::Content;
use super::engine::GridEngine;
use super::item_id::ItemId;
use super::state;

const DRAG_DEADBAND_DISTANCE: f32 = 10.0;
const RESIZE_LEEWAY: f32 = 8.0;

/// How cell height is determined in the grid.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum CellHeight {
    /// Cells are square — height equals the computed cell width.
    #[default]
    Auto,
    /// Each row has a fixed pixel height.
    Fixed(f32),
}

/// An event produced when an item is moved by dragging.
#[derive(Debug, Clone, Copy)]
pub struct MoveEvent {
    /// The item that was moved.
    pub id: ItemId,
    /// The new column position.
    pub x: u16,
    /// The new row position.
    pub y: u16,
}

/// An event produced when an item is resized by dragging an edge.
#[derive(Debug, Clone, Copy)]
pub struct ResizeEvent {
    /// The item that was resized.
    pub id: ItemId,
    /// The new width in columns.
    pub w: u16,
    /// The new height in rows.
    pub h: u16,
}

/// A grid-based layout widget inspired by [GridStack.js](https://gridstackjs.com/).
///
/// Items are placed on a discrete grid with a fixed number of columns. Each item
/// occupies an integer-sized rectangle `(x, y, w, h)` in grid coordinates.
/// Users can drag items by their title bars to move them, or drag the bottom-right
/// edges to resize them.
///
/// Unlike [`PaneGrid`], which uses recursive binary splits, `GridStack` uses
/// explicit integer coordinates. This allows arbitrary layouts including
/// L-shaped arrangements and items of varying sizes.
///
/// The widget does **not** mutate the engine state directly. Instead, it emits
/// [`MoveEvent`] and [`ResizeEvent`] messages that the application handles in
/// its `update` function.
///
/// # Example
///
/// ```ignore
/// use sweeten::widget::grid_stack::{self, GridStack, Content, TitleBar};
///
/// let grid = GridStack::new(&state, |id, item| {
///     Content::new(text(&item.label))
///         .title_bar(TitleBar::new(text("Title")).padding(5))
/// })
/// .spacing(10)
/// .on_click(Message::Clicked)
/// .on_move(Message::Moved)
/// .on_resize(Message::Resized);
/// ```
///
/// [`PaneGrid`]: https://docs.iced.rs/iced/widget/pane_grid/struct.PaneGrid.html
pub struct GridStack<
    'a,
    Message,
    Theme = crate::Theme,
    Renderer = crate::Renderer,
> where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    engine: &'a GridEngine,
    items: Vec<ItemId>,
    contents: Vec<Content<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
    spacing: f32,
    cell_height: CellHeight,
    on_click: Option<Box<dyn Fn(ItemId) -> Message + 'a>>,
    on_move: Option<Box<dyn Fn(MoveEvent) -> Message + 'a>>,
    on_resize: Option<Box<dyn Fn(ResizeEvent) -> Message + 'a>>,
    class: <Theme as Catalog>::Class<'a>,
    last_mouse_interaction: Option<mouse::Interaction>,
}

impl<'a, Message, Theme, Renderer> GridStack<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    /// Creates a [`GridStack`] with the given [`State`] and view function.
    ///
    /// The view function is called once for each item in the grid, receiving
    /// the item's [`ItemId`] and a reference to its user data.
    ///
    /// [`State`]: super::State
    pub fn new<T>(
        state: &'a state::State<T>,
        view: impl Fn(ItemId, &'a T) -> Content<'a, Message, Theme, Renderer>,
    ) -> Self {
        let items: Vec<ItemId> = state.iter().map(|(id, _)| id).collect();
        let contents: Vec<_> =
            state.iter().map(|(id, data)| view(id, data)).collect();

        Self {
            engine: state.engine(),
            items,
            contents,
            width: Length::Fill,
            height: Length::Shrink,
            spacing: 0.0,
            cell_height: CellHeight::default(),
            on_click: None,
            on_move: None,
            on_resize: None,
            class: <Theme as Catalog>::default(),
            last_mouse_interaction: None,
        }
    }

    /// Sets the width of the [`GridStack`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`GridStack`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the spacing between grid items, in pixels.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets how cell height is computed.
    ///
    /// - [`CellHeight::Auto`] — cells are square (height = width)
    /// - [`CellHeight::Fixed`] — each row has a fixed pixel height
    pub fn cell_height(mut self, cell_height: CellHeight) -> Self {
        self.cell_height = cell_height;
        self
    }

    /// Sets the message that will be produced when an item is clicked.
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(ItemId) -> Message,
    {
        self.on_click = Some(Box::new(f));
        self
    }

    /// Enables move interactions (drag to reposition).
    ///
    /// When enabled, items can be moved by dragging their title bar.
    pub fn on_move<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(MoveEvent) -> Message,
    {
        self.on_move = Some(Box::new(f));
        self
    }

    /// Enables resize interactions (drag edges to resize).
    ///
    /// When enabled, items can be resized by dragging their right or bottom
    /// edges.
    pub fn on_resize<F>(mut self, f: F) -> Self
    where
        F: 'a + Fn(ResizeEvent) -> Message,
    {
        self.on_resize = Some(Box::new(f));
        self
    }

    /// Sets the style of the [`GridStack`].
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        <Theme as Catalog>::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
        self
    }

    /// Computes the cell dimensions from the given total width.
    fn cell_dimensions(&self, total_width: f32) -> (f32, f32) {
        let cols = f32::from(self.engine.columns());
        let cell_w = (total_width - (cols - 1.0) * self.spacing) / cols;
        let cell_h = match self.cell_height {
            CellHeight::Auto => cell_w,
            CellHeight::Fixed(h) => h,
        };
        (cell_w, cell_h)
    }

    /// Computes the total grid height from the engine state and cell dimensions.
    fn grid_height(&self, cell_h: f32) -> f32 {
        let rows = f32::from(self.engine.get_row());
        if rows == 0.0 {
            0.0
        } else {
            rows * cell_h + (rows - 1.0) * self.spacing
        }
    }

    /// Converts a pixel rectangle from `item_regions`-style tuple to a [`Rectangle`].
    fn pixel_rect(region: (f32, f32, f32, f32)) -> Rectangle {
        Rectangle {
            x: region.0,
            y: region.1,
            width: region.2,
            height: region.3,
        }
    }
}

/// Ephemeral action state stored in the widget tree.
#[derive(Debug, Default, Clone)]
enum Action {
    /// No interaction in progress.
    #[default]
    Idle,
    /// The user is dragging an item to move it.
    Moving {
        /// The item being moved.
        id: ItemId,
        /// The cursor position when the drag started.
        origin: Point,
        /// Offset from the item's top-left corner to the grab point.
        /// Used to render the item freely under the cursor during drag.
        grab_offset: Vector,
        /// The item's grid position when the drag started.
        start_x: u16,
        start_y: u16,
        /// Cell dimensions at drag start (for pixel-to-grid conversion).
        cell_w: f32,
        cell_h: f32,
    },
    /// The user is dragging an edge to resize an item.
    Resizing {
        /// The item being resized.
        id: ItemId,
        /// The cursor position when the drag started.
        origin: Point,
        /// The item's grid size when the drag started.
        start_w: u16,
        start_h: u16,
        /// Cell dimensions at drag start.
        cell_w: f32,
        cell_h: f32,
    },
}

#[derive(Default)]
struct Memory {
    action: Action,
    order: Vec<ItemId>,
    last_hovered: Option<usize>,
    animations: ItemAnimations,
}

/// Per-item position animations for smooth transitions.
#[derive(Debug, Clone)]
struct ItemAnimations {
    /// Animated X offset for each item (pixels, from layout position).
    offsets_x: HashMap<ItemId, Animation<f32>>,
    /// Animated Y offset for each item (pixels, from layout position).
    offsets_y: HashMap<ItemId, Animation<f32>>,
    /// Last known pixel position for each item (top-left corner).
    last_positions: HashMap<ItemId, (f32, f32)>,
    /// Ghost placeholder opacity animation.
    ghost_opacity: Animation<bool>,
    /// The item that was being dragged when the ghost was last shown.
    ghost_item: Option<ItemId>,
    /// Current time instant for interpolation.
    now: Option<Instant>,
}

impl Default for ItemAnimations {
    fn default() -> Self {
        Self {
            offsets_x: HashMap::new(),
            offsets_y: HashMap::new(),
            last_positions: HashMap::new(),
            ghost_opacity: Animation::new(false),
            ghost_item: None,
            now: None,
        }
    }
}

impl ItemAnimations {
    fn new_animation(value: f32) -> Animation<f32> {
        Animation::new(value)
            .quick()
            .easing(crate::core::animation::Easing::EaseOut)
    }

    fn new_ghost_animation(value: bool) -> Animation<bool> {
        Animation::new(value)
            .quick()
            .easing(crate::core::animation::Easing::EaseOut)
    }

    /// Returns true if any item animation is in progress.
    fn is_animating(&self, now: Instant) -> bool {
        self.offsets_x.values().any(|anim| anim.is_animating(now))
            || self.offsets_y.values().any(|anim| anim.is_animating(now))
            || self.ghost_opacity.is_animating(now)
    }

    /// Update animations based on new item positions from layout.
    /// `regions` maps ItemId -> (px_x, px_y, pw, ph).
    /// `dragged_id` is the item currently being dragged (should not be animated).
    #[allow(clippy::type_complexity)]
    fn update_positions(
        &mut self,
        regions: &[(ItemId, (f32, f32, f32, f32))],
        dragged_id: Option<ItemId>,
        now: Instant,
    ) {
        for &(id, (px, py, _, _)) in regions {
            // Don't animate the item being dragged
            if dragged_id == Some(id) {
                self.last_positions.insert(id, (px, py));
                continue;
            }

            if let Some(&(old_x, old_y)) = self.last_positions.get(&id) {
                let dx = old_x - px;
                let dy = old_y - py;

                // Only start a new animation if the position actually changed
                // by a meaningful amount (> 0.5px to avoid float noise).
                if dx.abs() > 0.5 || dy.abs() > 0.5 {
                    let anim_x = Self::new_animation(dx).go(0.0, now);
                    let anim_y = Self::new_animation(dy).go(0.0, now);
                    self.offsets_x.insert(id, anim_x);
                    self.offsets_y.insert(id, anim_y);
                }
            }

            self.last_positions.insert(id, (px, py));
        }

        // Clean up stale entries for items that no longer exist.
        let current_ids: std::collections::HashSet<ItemId> =
            regions.iter().map(|(id, _)| *id).collect();
        self.offsets_x.retain(|id, _| current_ids.contains(id));
        self.offsets_y.retain(|id, _| current_ids.contains(id));
        self.last_positions.retain(|id, _| current_ids.contains(id));
    }

    /// Get the current interpolated offset for an item.
    fn get_offset(&self, id: ItemId, now: Instant) -> Vector {
        let x = self
            .offsets_x
            .get(&id)
            .filter(|anim| anim.is_animating(now))
            .map(|anim| anim.interpolate_with(|v| v, now))
            .unwrap_or(0.0);
        let y = self
            .offsets_y
            .get(&id)
            .filter(|anim| anim.is_animating(now))
            .map(|anim| anim.interpolate_with(|v| v, now))
            .unwrap_or(0.0);
        Vector::new(x, y)
    }

    /// Start the ghost fade-in animation.
    fn show_ghost(&mut self, id: ItemId, now: Instant) {
        if self.ghost_item != Some(id) {
            self.ghost_opacity = Self::new_ghost_animation(false).go(true, now);
            self.ghost_item = Some(id);
        }
    }

    /// Hide the ghost (reset state).
    fn hide_ghost(&mut self) {
        if self.ghost_item.is_some() {
            self.ghost_item = None;
            self.ghost_opacity = Self::new_ghost_animation(false);
        }
    }

    /// Get the current ghost opacity (0.0 to 1.0).
    fn ghost_alpha(&self, now: Instant) -> f32 {
        self.ghost_opacity.interpolate(0.0, 1.0, now)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for GridStack<'_, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Memory>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Memory::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.contents.iter().map(Content::state).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let Memory { order, .. } = tree.state.downcast_ref();

        // ItemId is monotonically increasing and iterated by Ord (BTreeMap),
        // so new states always appear at the end. We can remove states for
        // items that no longer exist, then diff_children_custom will reconcile.
        let mut i = 0;
        let mut j = 0;
        tree.children.retain(|_| {
            let retain = self.items.get(i) == order.get(j);
            if retain {
                i += 1;
            }
            j += 1;
            retain
        });

        tree.diff_children_custom(
            &self.contents,
            |state, content| content.diff(state),
            Content::state,
        );

        let Memory { order, .. } = tree.state.downcast_mut();
        order.clone_from(&self.items);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        // Resolve the width first. The height depends on cell dimensions
        // which depend on width.
        let resolved_width = match self.width {
            Length::Fill | Length::FillPortion(_) => max.width,
            Length::Shrink => max.width, // grid fills available width
            Length::Fixed(w) => w.min(max.width),
        };

        let (cell_w, cell_h) = self.cell_dimensions(resolved_width);
        let grid_h = self.grid_height(cell_h);

        let resolved_height = match self.height {
            Length::Fill | Length::FillPortion(_) => max.height,
            Length::Shrink => grid_h.min(max.height),
            Length::Fixed(h) => h.min(max.height),
        };

        let bounds = Size::new(resolved_width, resolved_height);

        // Use custom item_regions computation that respects CellHeight
        let regions = self.compute_regions(resolved_width, cell_w, cell_h);

        // Update animation state with the new positions.
        let memory = tree.state.downcast_mut::<Memory>();
        let dragged_id = match &memory.action {
            Action::Moving { id, .. } | Action::Resizing { id, .. } => {
                Some(*id)
            }
            Action::Idle => None,
        };
        let now = memory.animations.now.unwrap_or_else(Instant::now);
        memory
            .animations
            .update_positions(&regions, dragged_id, now);

        let children = self
            .items
            .iter()
            .zip(&mut self.contents)
            .zip(tree.children.iter_mut())
            .map(|((id, content), tree)| {
                let region =
                    regions.iter().find(|(rid, _)| rid == id).map(|(_, r)| *r);

                if let Some(region) = region {
                    let rect = Self::pixel_rect(region);
                    let size = Size::new(rect.width, rect.height);

                    let node = content.layout(
                        tree,
                        renderer,
                        &layout::Limits::new(size, size),
                    );
                    node.move_to(Point::new(rect.x, rect.y))
                } else {
                    layout::Node::new(Size::ZERO)
                }
            })
            .collect();

        layout::Node::with_children(bounds, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.contents
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((content, state), layout)| {
                    content.operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Memory { action, .. } = tree.state.downcast_mut();

        let picked_item = match action {
            Action::Moving { id, .. } | Action::Resizing { id, .. } => {
                Some(*id)
            }
            Action::Idle => None,
        };

        // Propagate events to contents
        for ((id, content), (tree, layout)) in self
            .items
            .iter()
            .copied()
            .zip(&mut self.contents)
            .zip(tree.children.iter_mut().zip(layout.children()))
        {
            let is_picked = picked_item == Some(id);
            content.update(
                tree, event, layout, cursor, renderer, shell, viewport,
                is_picked,
            );
        }

        let bounds = layout.bounds();
        let Memory {
            action, animations, ..
        } = tree.state.downcast_mut();

        match event {
            Event::Window(window::Event::RedrawRequested(now)) => {
                animations.now = Some(*now);

                // Request another frame if animations are still in progress.
                if animations.is_animating(*now) {
                    shell.request_redraw();
                }

                // Manage ghost animation based on drag state.
                match action {
                    Action::Moving { id, origin, .. } => {
                        if cursor.position().is_some_and(|pos| {
                            pos.distance(*origin) > DRAG_DEADBAND_DISTANCE
                        }) {
                            animations.show_ghost(*id, *now);
                        }
                    }
                    Action::Idle | Action::Resizing { .. } => {
                        animations.hide_ghost();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if let Some(cursor_position) = cursor.position_over(bounds) {
                    shell.capture_event();

                    let (cell_w, cell_h) = self.cell_dimensions(bounds.width);

                    // Check for resize first (bottom-right edge detection)
                    if self.on_resize.is_some()
                        && let Some(resize_id) = self.find_resize_target(
                            layout,
                            cursor_position,
                            bounds,
                        )
                        && let Some(item) = self.engine.get(resize_id)
                        && !item.locked
                    {
                        *action = Action::Resizing {
                            id: resize_id,
                            origin: cursor_position,
                            start_w: item.w,
                            start_h: item.h,
                            cell_w,
                            cell_h,
                        };
                        return;
                    }

                    // Check for click/drag on an item
                    self.click_item(
                        action,
                        layout,
                        cursor_position,
                        shell,
                        cell_w,
                        cell_h,
                    );
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. })
            | Event::Touch(touch::Event::FingerLost { .. }) => {
                *action = Action::Idle;
                animations.hide_ghost();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. }) => match action {
                Action::Moving {
                    id,
                    origin,
                    start_x,
                    start_y,
                    cell_w,
                    cell_h,
                    ..
                } => {
                    if let Some(on_move) = &self.on_move
                        && let Some(cursor_position) = cursor.position()
                        && cursor_position.distance(*origin)
                            > DRAG_DEADBAND_DISTANCE
                    {
                        let dx = cursor_position.x - origin.x;
                        let dy = cursor_position.y - origin.y;
                        let step_w = *cell_w + self.spacing;
                        let step_h = *cell_h + self.spacing;
                        let grid_dx = (dx / step_w).round() as i32;
                        let grid_dy = (dy / step_h).round() as i32;
                        let new_x = (*start_x as i32 + grid_dx).max(0) as u16;
                        let new_y = (*start_y as i32 + grid_dy).max(0) as u16;

                        // Start ghost animation when drag begins
                        let now = animations.now.unwrap_or_else(Instant::now);
                        animations.show_ghost(*id, now);

                        shell.publish(on_move(MoveEvent {
                            id: *id,
                            x: new_x,
                            y: new_y,
                        }));
                    }
                    shell.request_redraw();
                }
                Action::Resizing {
                    id,
                    origin,
                    start_w,
                    start_h,
                    cell_w,
                    cell_h,
                } => {
                    if let Some(on_resize) = &self.on_resize
                        && let Some(cursor_position) = cursor.position()
                    {
                        let dx = cursor_position.x - origin.x;
                        let dy = cursor_position.y - origin.y;
                        let step_w = *cell_w + self.spacing;
                        let step_h = *cell_h + self.spacing;
                        let grid_dw = (dx / step_w).round() as i32;
                        let grid_dh = (dy / step_h).round() as i32;
                        let new_w = (*start_w as i32 + grid_dw).max(1) as u16;
                        let new_h = (*start_h as i32 + grid_dh).max(1) as u16;

                        shell.publish(on_resize(ResizeEvent {
                            id: *id,
                            w: new_w,
                            h: new_h,
                        }));
                    }
                    shell.request_redraw();
                }
                Action::Idle => {}
            },
            _ => {}
        }

        // Track which item the cursor hovers so we can request redraws when it
        // changes.  This keeps show/hide of title-bar controls in sync.
        {
            let hovered_index = cursor.position_over(bounds).and_then(|pos| {
                layout
                    .children()
                    .enumerate()
                    .find(|(_, child)| child.bounds().contains(pos))
                    .map(|(i, _)| i)
            });

            let memory = tree.state.downcast_mut::<Memory>();
            if memory.last_hovered != hovered_index {
                memory.last_hovered = hovered_index;
                shell.request_redraw();
            }
        }

        // Detect mouse interaction changes (cursor type) so we request redraws
        // for hover effects like the resize cursor or grab cursor.
        if shell.redraw_request() != window::RedrawRequest::NextFrame {
            let action = &tree.state.downcast_ref::<Memory>().action;

            let interaction = self
                .grid_interaction(action, layout, cursor)
                .or_else(|| {
                    self.items
                        .iter()
                        .zip(&self.contents)
                        .zip(layout.children())
                        .find_map(|((_, content), layout)| {
                            content.grid_interaction(
                                layout,
                                cursor,
                                self.on_move.is_some(),
                            )
                        })
                })
                .unwrap_or(mouse::Interaction::None);

            if let Event::Window(window::Event::RedrawRequested(_)) = event {
                self.last_mouse_interaction = Some(interaction);
            } else if self
                .last_mouse_interaction
                .is_some_and(|last| last != interaction)
            {
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Memory { action, .. } = tree.state.downcast_ref();

        match action {
            Action::Moving { origin, .. } => {
                if let Some(pos) = cursor.position()
                    && pos.distance(*origin) > DRAG_DEADBAND_DISTANCE
                {
                    return mouse::Interaction::Grabbing;
                }
                return mouse::Interaction::Grab;
            }
            Action::Resizing { .. } => {
                return mouse::Interaction::ResizingDiagonallyDown;
            }
            Action::Idle => {}
        }

        let bounds = layout.bounds();

        // Check for resize cursor on edges
        if self.on_resize.is_some()
            && let Some(cursor_position) = cursor.position_over(bounds)
            && self
                .find_resize_target(layout, cursor_position, bounds)
                .is_some()
        {
            return mouse::Interaction::ResizingDiagonallyDown;
        }

        // Check content interactions
        let drag_enabled = self.on_move.is_some();

        self.items
            .iter()
            .zip(&self.contents)
            .zip(&tree.children)
            .zip(layout.children())
            .map(|(((_, content), tree), layout)| {
                content.mouse_interaction(
                    tree,
                    layout,
                    cursor,
                    viewport,
                    renderer,
                    drag_enabled,
                )
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Memory {
            action, animations, ..
        } = tree.state.downcast_ref();
        let now = animations.now.unwrap_or_else(Instant::now);

        let picked_item = match action {
            Action::Moving {
                id,
                origin,
                grab_offset,
                ..
            } => cursor
                .position()
                .filter(|pos| pos.distance(*origin) > DRAG_DEADBAND_DISTANCE)
                .map(|_| (*id, *grab_offset)),
            _ => None,
        };

        let resizing_id = match action {
            Action::Resizing { id, .. } => Some(*id),
            _ => None,
        };

        let item_cursor = if picked_item.is_some() {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };

        let mut render_picked = None;

        for (((id, content), tree), item_layout) in self
            .items
            .iter()
            .copied()
            .zip(&self.contents)
            .zip(&tree.children)
            .zip(layout.children())
        {
            match picked_item {
                Some((dragging, grab_offset)) if id == dragging => {
                    render_picked =
                        Some(((content, tree), item_layout, grab_offset));
                }
                _ => {
                    // When resizing an item, force its controls to stay
                    // visible by providing a cursor over its bounds.
                    let draw_cursor = if resizing_id == Some(id) {
                        let b = item_layout.bounds();
                        mouse::Cursor::Available(Point::new(
                            b.x + b.width / 2.0,
                            b.y + b.height / 2.0,
                        ))
                    } else {
                        item_cursor
                    };

                    // Apply animated offset for smooth transitions.
                    let offset = animations.get_offset(id, now);

                    if offset.x != 0.0 || offset.y != 0.0 {
                        renderer.with_translation(offset, |renderer| {
                            content.draw(
                                tree,
                                renderer,
                                theme,
                                defaults,
                                item_layout,
                                draw_cursor,
                                viewport,
                            );
                        });
                    } else {
                        content.draw(
                            tree,
                            renderer,
                            theme,
                            defaults,
                            item_layout,
                            draw_cursor,
                            viewport,
                        );
                    }
                }
            }
        }

        // Draw resize grip indicator on hovered, non-locked items when
        // resize is enabled and the style provides a grip appearance.
        let grid_style = Catalog::style(theme, &self.class);

        if self.on_resize.is_some()
            && let Some(ref grip) = grid_style.resize_grip
            && let Some(cursor_position) = item_cursor.position()
        {
            for (id, item_layout) in
                self.items.iter().copied().zip(layout.children())
            {
                let item_bounds = item_layout.bounds();
                if !item_bounds.contains(cursor_position) {
                    continue;
                }
                if self.engine.get(id).is_some_and(|item| item.locked) {
                    continue;
                }
                if picked_item.is_some_and(|(pid, _)| pid == id) {
                    continue;
                }

                // Draw a triangular grip pattern at the bottom-right:
                //       .
                //     . .
                //   . . .
                let margin = 6.0;
                let gap = 4.0;
                let anchor_x = item_bounds.x + item_bounds.width - margin;
                let anchor_y = item_bounds.y + item_bounds.height - margin;

                for row in 0..3_u8 {
                    for col in 0..=row {
                        let x = anchor_x - (col as f32) * gap;
                        let y = anchor_y - ((2 - row) as f32) * gap;

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x,
                                    y,
                                    width: grip.dot_size,
                                    height: grip.dot_size,
                                },
                                border: Border {
                                    radius: (grip.dot_size / 2.0).into(),
                                    ..Border::default()
                                },
                                ..renderer::Quad::default()
                            },
                            Background::Color(grip.color),
                        );
                    }
                }
            }
        }

        // Render the picked item last, floating freely under the cursor.
        // The item follows the mouse exactly (organic feel) while the engine
        // handles snapped grid positions for other items underneath.
        if let Some(((content, tree), item_layout, grab_offset)) = render_picked
            && let Some(cursor_position) = cursor.position()
        {
            // Draw a translucent ghost rectangle at the engine's snap position
            // (where the item would land if released now), with animated opacity.
            // The ghost is clipped to avoid overlapping items that are still
            // animating away from the ghost area.
            let ghost_target = item_layout.bounds();
            let ghost_alpha = animations.ghost_alpha(now);

            let dragged_id = picked_item.unwrap().0;
            let animating: Vec<(Rectangle, Vector)> = self
                .items
                .iter()
                .copied()
                .zip(layout.children())
                .filter_map(|(id, child_layout)| {
                    if id == dragged_id {
                        return None;
                    }
                    let offset = animations.get_offset(id, now);
                    if offset.x == 0.0 && offset.y == 0.0 {
                        return None;
                    }
                    Some((child_layout.bounds(), offset))
                })
                .collect();

            let ghost_bounds =
                clip_ghost_for_animating_items(ghost_target, &animating);

            renderer.fill_quad(
                renderer::Quad {
                    bounds: ghost_bounds,
                    border: Border {
                        color: grid_style
                            .hovered_region
                            .border
                            .color
                            .scale_alpha(ghost_alpha),
                        ..grid_style.hovered_region.border
                    },
                    ..renderer::Quad::default()
                },
                grid_style
                    .hovered_region
                    .background
                    .scale_alpha(ghost_alpha),
            );

            // Draw the floating item under the cursor.
            let layout_pos = ghost_target.position();
            let target = Point::new(
                cursor_position.x - grab_offset.x,
                cursor_position.y - grab_offset.y,
            );
            let translation =
                Vector::new(target.x - layout_pos.x, target.y - layout_pos.y);

            renderer.with_translation(translation, |renderer| {
                renderer.with_layer(ghost_target, |renderer| {
                    content.draw(
                        tree,
                        renderer,
                        theme,
                        defaults,
                        item_layout,
                        mouse::Cursor::Unavailable,
                        viewport,
                    );
                });
            });
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let children = self
            .contents
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((content, state), layout)| {
                content.overlay(state, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!children.is_empty()).then(|| Group::with_children(children).overlay())
    }
}

impl<'a, Message, Theme, Renderer> GridStack<'a, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: core::Renderer,
{
    /// Compute item regions using the cell_height setting.
    fn compute_regions(
        &self,
        total_width: f32,
        cell_w: f32,
        cell_h: f32,
    ) -> Vec<(ItemId, (f32, f32, f32, f32))> {
        self.engine
            .items()
            .map(|item| {
                let x = f32::from(item.x);
                let y = f32::from(item.y);
                let w = f32::from(item.w);
                let h = f32::from(item.h);

                let px = (x * cell_w + x * self.spacing).round();
                let py = (y * cell_h + y * self.spacing).round();
                let pw = (w * cell_w + (w - 1.0) * self.spacing).round();
                let ph = (h * cell_h + (h - 1.0) * self.spacing).round();

                // Clamp width to not exceed total_width
                let pw = pw.min(total_width - px);

                (item.id, (px, py, pw, ph))
            })
            .collect()
    }

    /// Find an item whose right or bottom edge is near the cursor for resizing.
    fn find_resize_target(
        &self,
        _layout: Layout<'_>,
        cursor_position: Point,
        bounds: Rectangle,
    ) -> Option<ItemId> {
        let (cell_w, cell_h) = self.cell_dimensions(bounds.width);
        let regions = self.compute_regions(bounds.width, cell_w, cell_h);

        for (id, region) in &regions {
            // Skip locked items — they cannot be resized.
            if self.engine.get(*id).is_some_and(|item| item.locked) {
                continue;
            }

            let rect = Self::pixel_rect(*region);
            // Translate to absolute coordinates
            let abs_rect = Rectangle {
                x: rect.x + bounds.x,
                y: rect.y + bounds.y,
                width: rect.width,
                height: rect.height,
            };

            let near_right =
                (cursor_position.x - (abs_rect.x + abs_rect.width)).abs()
                    < RESIZE_LEEWAY
                    && cursor_position.y >= abs_rect.y
                    && cursor_position.y <= abs_rect.y + abs_rect.height;

            let near_bottom =
                (cursor_position.y - (abs_rect.y + abs_rect.height)).abs()
                    < RESIZE_LEEWAY
                    && cursor_position.x >= abs_rect.x
                    && cursor_position.x <= abs_rect.x + abs_rect.width;

            if near_right || near_bottom {
                return Some(*id);
            }
        }

        None
    }

    /// Computes the current mouse interaction based on the action state and
    /// cursor position, without consulting child widgets.
    fn grid_interaction(
        &self,
        action: &Action,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) -> Option<mouse::Interaction> {
        match action {
            Action::Moving { origin, .. } => {
                if cursor.position().is_some_and(|pos| {
                    pos.distance(*origin) > DRAG_DEADBAND_DISTANCE
                }) {
                    return Some(mouse::Interaction::Grabbing);
                }
                return Some(mouse::Interaction::Grab);
            }
            Action::Resizing { .. } => {
                return Some(mouse::Interaction::ResizingDiagonallyDown);
            }
            Action::Idle => {}
        }

        let bounds = layout.bounds();

        if self.on_resize.is_some()
            && let Some(cursor_position) = cursor.position_over(bounds)
            && self
                .find_resize_target(layout, cursor_position, bounds)
                .is_some()
        {
            return Some(mouse::Interaction::ResizingDiagonallyDown);
        }

        None
    }

    /// Handles a click on an item, potentially starting a drag.
    fn click_item(
        &self,
        action: &mut Action,
        layout: Layout<'_>,
        cursor_position: Point,
        shell: &mut Shell<'_, Message>,
        cell_w: f32,
        cell_h: f32,
    ) {
        let clicked = self
            .items
            .iter()
            .copied()
            .zip(&self.contents)
            .zip(layout.children())
            .find(|((_, _), layout)| layout.bounds().contains(cursor_position));

        if let Some(((id, content), item_layout)) = clicked {
            if let Some(on_click) = &self.on_click {
                shell.publish(on_click(id));
            }

            if self.on_move.is_some()
                && content.can_be_dragged_at(item_layout, cursor_position)
                && let Some(item) = self.engine.get(id)
                && !item.locked
            {
                let item_pos = item_layout.bounds().position();
                let grab_offset = Vector::new(
                    cursor_position.x - item_pos.x,
                    cursor_position.y - item_pos.y,
                );

                *action = Action::Moving {
                    id,
                    origin: cursor_position,
                    grab_offset,
                    start_x: item.x,
                    start_y: item.y,
                    cell_w,
                    cell_h,
                };
            }
        }
    }
}

impl<'a, Message, Theme, Renderer> From<GridStack<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: core::Renderer + 'a,
{
    fn from(grid_stack: GridStack<'a, Message, Theme, Renderer>) -> Self {
        Element::new(grid_stack)
    }
}

/// The appearance of a [`GridStack`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the grid area.
    pub background: Option<Background>,
    /// The [`Border`] around the grid area.
    pub border: Border,
    /// The highlight shown when an item is being dragged over another.
    pub hovered_region: Highlight,
    /// The appearance of the resize grip indicator shown on hovered items.
    ///
    /// Set to `None` to disable the resize grip.
    pub resize_grip: Option<ResizeGrip>,
}

/// A highlight region appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Highlight {
    /// The [`Background`] of the highlighted region.
    pub background: Background,
    /// The [`Border`] of the highlighted region.
    pub border: Border,
}

/// The appearance of the resize grip indicator drawn at the bottom-right
/// corner of a hovered grid item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeGrip {
    /// The color of the grip dots.
    pub color: Color,
    /// The size of each dot in pixels.
    pub dot_size: f32,
}

/// The theme catalog for a [`GridStack`].
pub trait Catalog: container::Catalog {
    /// The item class of this [`Catalog`].
    type Class<'a>;

    /// The default class produced by this [`Catalog`].
    fn default<'a>() -> <Self as Catalog>::Class<'a>;

    /// The [`Style`] of a class.
    fn style(&self, class: &<Self as Catalog>::Class<'_>) -> Style;
}

/// A styling function for a [`GridStack`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> StyleFn<'a, Self> {
        Box::new(default_style)
    }

    fn style(&self, class: &StyleFn<'_, Self>) -> Style {
        class(self)
    }
}

/// Clips a ghost rectangle to avoid overlapping with items that are
/// animating away from the ghost's area.
///
/// Each entry in `animating_items` is `(layout_bounds, animation_offset)`,
/// where `layout_bounds` is the item's final (layout) position, and
/// `animation_offset` is the current visual offset from that position (the
/// offset starts large and converges to zero as the animation completes).
///
/// The returned rectangle is the ghost shrunk so it never visually overlaps
/// any of the animating items.
fn clip_ghost_for_animating_items(
    ghost: Rectangle,
    animating_items: &[(Rectangle, Vector)],
) -> Rectangle {
    let mut bounds = ghost;

    for &(layout_rect, offset) in animating_items {
        if offset.x == 0.0 && offset.y == 0.0 {
            continue;
        }

        // The item's visual rectangle: layout position + current offset.
        let visual = Rectangle {
            x: layout_rect.x + offset.x,
            y: layout_rect.y + offset.y,
            width: layout_rect.width,
            height: layout_rect.height,
        };

        // Skip if no overlap between current ghost bounds and this item.
        if bounds.y >= visual.y + visual.height
            || bounds.y + bounds.height <= visual.y
            || bounds.x >= visual.x + visual.width
            || bounds.x + bounds.width <= visual.x
        {
            continue;
        }

        // Clip vertically based on offset direction.
        if offset.y < 0.0 {
            // Item visual is above its layout position, sliding DOWN.
            // It occupies space in the lower portion of the ghost area.
            // Reveal the ghost from the top: clip the bottom edge.
            let available = (visual.y - bounds.y).max(0.0);
            bounds.height = bounds.height.min(available);
        } else if offset.y > 0.0 {
            // Item visual is below its layout position, sliding UP.
            // It occupies space in the upper portion of the ghost area.
            // Reveal the ghost from the bottom: clip the top edge.
            let new_top = (visual.y + visual.height)
                .max(bounds.y)
                .min(bounds.y + bounds.height);
            bounds.height -= new_top - bounds.y;
            bounds.y = new_top;
        }

        // Clip horizontally based on offset direction.
        if offset.x < 0.0 {
            // Item visual is left of its layout position, sliding RIGHT.
            // Reveal the ghost from the left: clip the right edge.
            let available = (visual.x - bounds.x).max(0.0);
            bounds.width = bounds.width.min(available);
        } else if offset.x > 0.0 {
            // Item visual is right of its layout position, sliding LEFT.
            // Reveal the ghost from the right: clip the left edge.
            let new_left = (visual.x + visual.width)
                .max(bounds.x)
                .min(bounds.x + bounds.width);
            bounds.width -= new_left - bounds.x;
            bounds.x = new_left;
        }
    }

    // Ensure non-negative dimensions.
    bounds.width = bounds.width.max(0.0);
    bounds.height = bounds.height.max(0.0);

    bounds
}

/// The default style of a [`GridStack`].
pub fn default_style(_theme: &Theme) -> Style {
    Style {
        background: None,
        border: Border::default(),
        hovered_region: Highlight {
            background: Background::Color(Color {
                a: 0.08,
                ..Color::BLACK
            }),
            border: Border {
                width: 1.0,
                color: Color {
                    a: 0.15,
                    ..Color::BLACK
                },
                radius: 6.0.into(),
            },
        },
        resize_grip: Some(ResizeGrip {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.25,
            },
            dot_size: 2.0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle {
            x,
            y,
            width: w,
            height: h,
        }
    }

    fn vec2(x: f32, y: f32) -> Vector {
        Vector::new(x, y)
    }

    #[test]
    fn ghost_unchanged_when_no_animating_items() {
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let result = clip_ghost_for_animating_items(ghost, &[]);
        assert_eq!(result, ghost);
    }

    #[test]
    fn ghost_unchanged_when_items_not_overlapping() {
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        // Item far below ghost, animating but not overlapping.
        let items = [(rect(0.0, 500.0, 280.0, 200.0), vec2(0.0, -50.0))];
        let result = clip_ghost_for_animating_items(ghost, &items);
        assert_eq!(result, ghost);
    }

    #[test]
    fn ghost_grows_from_top_as_item_slides_down() {
        // Ghost at (0, 0, 280, 200).
        // Item displaced from y=0 to y=208 (layout).
        // offset_y starts at -208 (visual at y=0), goes to 0 (visual at y=208).
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 208.0, 280.0, 200.0);

        // At animation start: offset_y = -208, visual at y=0 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -208.0))],
        );
        assert_eq!(result.height, 0.0);
        assert_eq!(result.y, 0.0);

        // At midpoint: offset_y = -104, visual at y=104.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -104.0))],
        );
        assert_eq!(result.height, 104.0);
        assert_eq!(result.y, 0.0);

        // Near end: offset_y = -8, visual at y=200 — no overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, -8.0))],
        );
        assert_eq!(result.height, 200.0);
        assert_eq!(result.y, 0.0);
    }

    #[test]
    fn ghost_grows_from_bottom_as_item_slides_up() {
        // Ghost at (0, 208, 280, 200) — from y=208 to y=408.
        // Item displaced from y=208 to y=0 (layout).
        // offset_y starts at 208 (visual at y=208), goes to 0 (visual at y=0).
        let ghost = rect(0.0, 208.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 0.0, 280.0, 200.0);

        // At animation start: offset_y = 208, visual at y=208 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, 208.0))],
        );
        assert_eq!(result.height, 0.0);

        // At midpoint: offset_y = 104, visual at y=104 — bottom of visual is 304.
        // Ghost from 208..408, visual from 104..304. Overlap = 208..304.
        // Ghost should show 304..408, height = 104.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(0.0, 104.0))],
        );
        assert!((result.y - 304.0).abs() < f32::EPSILON);
        assert!((result.height - 104.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ghost_grows_from_left_as_item_slides_right() {
        // Ghost at (0, 0, 280, 200).
        // Item displaced from x=0 to x=300 (layout).
        // offset_x starts at -300 (visual at x=0), goes to 0 (visual at x=300).
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(300.0, 0.0, 280.0, 200.0);

        // At animation start: offset_x = -300, visual at x=0 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(-300.0, 0.0))],
        );
        assert_eq!(result.width, 0.0);
        assert_eq!(result.x, 0.0);

        // At midpoint: offset_x = -150, visual at x=150.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(-150.0, 0.0))],
        );
        assert_eq!(result.width, 150.0);
        assert_eq!(result.x, 0.0);
    }

    #[test]
    fn ghost_grows_from_right_as_item_slides_left() {
        // Ghost at (300, 0, 280, 200).
        // Item displaced from x=300 to x=0 (layout).
        // offset_x starts at 300 (visual at x=300), goes to 0 (visual at x=0).
        let ghost = rect(300.0, 0.0, 280.0, 200.0);
        let layout_bounds = rect(0.0, 0.0, 280.0, 200.0);

        // At animation start: offset_x = 300, visual at x=300 — full overlap.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(300.0, 0.0))],
        );
        assert_eq!(result.width, 0.0);

        // At midpoint: offset_x = 150, visual at x=150 — visual right = 430.
        // Ghost from x=300..580. Overlap = 300..430.
        // Ghost should show 430..580, width = 150.
        let result = clip_ghost_for_animating_items(
            ghost,
            &[(layout_bounds, vec2(150.0, 0.0))],
        );
        assert!((result.x - 430.0).abs() < f32::EPSILON);
        assert!((result.width - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ghost_clipped_by_multiple_items() {
        // Ghost at (0, 0, 280, 400).
        // Item A sliding down from top of ghost.
        // Item B sliding up from bottom of ghost.
        let ghost = rect(0.0, 0.0, 280.0, 400.0);

        // Item A: layout at y=200, offset_y = -100, visual at y=100.
        // Clips ghost bottom to y=100.
        let item_a = (rect(0.0, 200.0, 280.0, 200.0), vec2(0.0, -100.0));

        // Item B: layout at y=-200, offset_y = 100, visual at y=-100, bottom=100.
        // Clips ghost top to y=100. But after item A already clipped bottom
        // to height=100 (ghost goes from 0..100), item B visual bottom is 100
        // which equals the ghost bottom, so overlap check: ghost 0..100,
        // visual -100..100 — they overlap. Clip top: new_top = 100.
        // ghost height = 0.
        let item_b = (rect(0.0, -200.0, 280.0, 200.0), vec2(0.0, 100.0));

        let result = clip_ghost_for_animating_items(ghost, &[item_a, item_b]);
        assert_eq!(result.height, 0.0);
    }

    #[test]
    fn ghost_full_size_when_item_animation_complete() {
        // Item has zero offset — animation finished.
        let ghost = rect(0.0, 0.0, 280.0, 200.0);
        let items = [(rect(0.0, 208.0, 280.0, 200.0), vec2(0.0, 0.0))];
        let result = clip_ghost_for_animating_items(ghost, &items);
        assert_eq!(result, ghost);
    }
}
