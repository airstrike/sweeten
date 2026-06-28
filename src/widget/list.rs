//! A virtualized list that only materializes visible items.
//!
//! Ported from iced's `feat/list-widget-redux` branch. The [`List`]
//! widget takes a [`Content<T>`] (the full data) and a view closure,
//! but only instantiates [`Element`]s for the items currently in the
//! viewport. This makes it suitable for large or unbounded data sets
//! where creating a widget per row would be too expensive.
//!
//! Modified from the `List` widget by Héctor Ramón Jiménez (hecrj),
//! originally on iced's `feat/list-widget-reloaded` branch.
//! Licensed under the MIT license.

use crate::core::layout;
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget;
use crate::core::widget::tree::{self, Tree};
use crate::core::window;
use crate::core::{
    self, Element, Event, Layout, Length, Pixels, Point, Rectangle, Shell,
    Size, Vector, Widget,
};

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::VecDeque;

/// A virtualized list widget.
///
/// Only the items visible in the current viewport are materialized
/// into [`Element`]s, so scrolling through thousands of rows stays
/// responsive. The backing data lives in a [`Content<T>`] that
/// tracks insertions, removals, and mutations so the widget tree
/// stays in sync without a full rebuild each frame.
#[allow(missing_debug_implementations)]
pub struct List<'a, T, Message, Theme, Renderer> {
    content: &'a Content<T>,
    spacing: f32,
    view_item:
        Box<dyn Fn(usize, &'a T) -> Element<'a, Message, Theme, Renderer> + 'a>,
    visible_elements: Vec<Element<'a, Message, Theme, Renderer>>,
}

impl<'a, T, Message, Theme, Renderer> List<'a, T, Message, Theme, Renderer> {
    /// Creates a new [`List`] backed by `content`, using `view_item`
    /// to materialize each visible row.
    pub fn new(
        content: &'a Content<T>,
        view_item: impl Fn(usize, &'a T) -> Element<'a, Message, Theme, Renderer>
        + 'a,
    ) -> Self {
        Self {
            content,
            spacing: 0.0,
            view_item: Box::new(view_item),
            visible_elements: Vec::new(),
        }
    }

    /// Sets the vertical spacing _between_ elements.
    ///
    /// Custom margins per element do not exist in iced. You should use this
    /// method instead! While less flexible, it helps you keep spacing between
    /// elements consistent.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }
}

struct State {
    last_limits: layout::Limits,
    visible_layouts: Vec<(usize, layout::Node, Tree)>,
    size: Size,
    offsets: Vec<f32>,
    widths: Vec<f32>,
    task: Task,
    visible_outdated: bool,
}

enum Task {
    Idle,
    Computing {
        current: usize,
        offsets: Vec<f32>,
        widths: Vec<f32>,
        size: Size,
    },
}

impl State {
    fn recompute(&mut self, size: usize) {
        let mut offsets = Vec::with_capacity(size + 1);
        offsets.push(0.0);

        self.task = Task::Computing {
            current: 0,
            offsets,
            widths: Vec::with_capacity(size),
            size: Size::ZERO,
        };
        self.visible_layouts.clear();
    }
}

impl<'a, T, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for List<'a, T, Message, Theme, Renderer>
where
    Renderer: core::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            last_limits: layout::Limits::NONE,
            visible_layouts: Vec::new(),
            size: Size::ZERO,
            offsets: vec![0.0],
            widths: Vec::new(),
            task: Task::Idle,
            visible_outdated: false,
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let loose_limits = limits.loose();

        if state.last_limits != loose_limits {
            state.last_limits = loose_limits;
            state.recompute(self.content.len());
        }

        let mut changes = self.content.changes.borrow_mut();

        match state.task {
            Task::Idle => {
                while let Some(change) = changes.pop_front() {
                    match change {
                        Change::Updated { original, current } => {
                            let mut new_element = (self.view_item)(
                                current,
                                &self.content.items[current],
                            );

                            let visible_index = state
                                .visible_layouts
                                .iter_mut()
                                .position(|(i, _, _)| *i == original);

                            let mut new_tree;

                            // Update if visible
                            let tree = if let Some(visible_index) =
                                visible_index
                            {
                                let (_i, _layout, tree) =
                                    &mut state.visible_layouts[visible_index];

                                tree.diff(new_element.as_widget_mut());
                                state.visible_outdated = true;

                                tree
                            } else {
                                new_tree = Tree::new(&new_element);
                                new_element.as_widget_mut().diff(&mut new_tree);

                                &mut new_tree
                            };

                            let new_layout = new_element
                                .as_widget_mut()
                                .layout(tree, renderer, &state.last_limits);

                            let new_size = new_layout.size();

                            let height_difference = new_size.height
                                - (state.offsets[original + 1]
                                    - state.offsets[original]);

                            for offset in &mut state.offsets[original + 1..] {
                                *offset += height_difference;
                            }

                            let original_width = state.widths[original];
                            state.widths[original] = new_size.width;

                            if let Some(visible_index) = visible_index {
                                state.visible_layouts[visible_index].1 =
                                    new_layout;

                                for (i, layout, _) in
                                    &mut state.visible_layouts[visible_index..]
                                {
                                    layout
                                        .move_to_mut((0.0, state.offsets[*i]));
                                }
                            } else if let Some(first_visible) =
                                state.visible_layouts.first()
                            {
                                let first_visible_index = first_visible.0;
                                if original < first_visible_index {
                                    for (i, layout, _) in
                                        &mut state.visible_layouts[..]
                                    {
                                        layout.move_to_mut((
                                            0.0,
                                            state.offsets[*i],
                                        ));
                                    }
                                }
                            }

                            state.size.height += height_difference;

                            if original_width == state.size.width {
                                state.size.width = state.widths.iter().fold(
                                    0.0,
                                    |current, candidate| {
                                        current.max(*candidate)
                                    },
                                );
                            }
                        }
                        Change::Removed { original, .. } => {
                            let height = state.offsets[original + 1]
                                - state.offsets[original];

                            let original_width = state.widths.remove(original);
                            let _ = state.offsets.remove(original + 1);

                            for offset in &mut state.offsets[original + 1..] {
                                *offset -= height;
                            }

                            // TODO: Smarter visible layout partial updates
                            state.visible_layouts.clear();

                            state.size.height -= height;

                            if original_width == state.size.width {
                                state.size.width = state.widths.iter().fold(
                                    0.0,
                                    |current, candidate| {
                                        current.max(*candidate)
                                    },
                                );
                            }
                        }
                        Change::Pushed { current, .. } => {
                            let mut new_element = (self.view_item)(
                                current,
                                &self.content.items[current],
                            );

                            let mut tree = Tree::new(&new_element);
                            new_element.as_widget_mut().diff(&mut tree);

                            let layout = new_element.as_widget_mut().layout(
                                &mut tree,
                                renderer,
                                &state.last_limits,
                            );

                            let size = layout.size();

                            state.widths.push(size.width);
                            state.offsets.push(
                                state.offsets.last().unwrap() + size.height,
                            );

                            state.size.width = state.size.width.max(size.width);
                            state.size.height += size.height;
                        }
                    }
                }
            }
            Task::Computing { .. } => {
                if !changes.is_empty() {
                    changes.clear();
                    state.recompute(self.content.len());
                }
            }
        }

        // Recompute if new
        {
            let mut is_new = self.content.is_new.borrow_mut();

            if *is_new {
                state.recompute(self.content.len());
                *is_new = false;
            }
        }

        match &mut state.task {
            Task::Idle => {}
            Task::Computing {
                current,
                size,
                widths,
                offsets,
            } => {
                const MAX_BATCH_SIZE: usize = 50;

                let end = (*current + MAX_BATCH_SIZE).min(self.content.len());

                let batch = &self.content.items[*current..end];

                let mut max_width = size.width;
                let mut accumulated_height =
                    offsets.last().copied().unwrap_or(0.0);

                for (i, item) in batch.iter().enumerate() {
                    let mut element = (self.view_item)(*current + i, item);
                    let mut tree = Tree::new(&element);
                    element.as_widget_mut().diff(&mut tree);

                    let layout = element
                        .as_widget_mut()
                        .layout(&mut tree, renderer, &state.last_limits)
                        .move_to((0.0, accumulated_height));

                    let bounds = layout.bounds();

                    max_width = max_width.max(bounds.width);
                    accumulated_height += bounds.height;

                    offsets.push(accumulated_height);
                    widths.push(bounds.width);
                }

                *size = Size::new(max_width, accumulated_height);

                if end < self.content.len() {
                    *current = end;
                } else {
                    state.offsets = std::mem::take(offsets);
                    state.widths = std::mem::take(widths);
                    state.size = std::mem::take(size);
                    state.task = Task::Idle;
                }
            }
        }

        let current_size = match &state.task {
            Task::Idle => state.size,
            Task::Computing { size, .. } => *size,
        };

        let intrinsic_size = Size::new(
            current_size.width,
            current_size.height
                + self.content.len().saturating_sub(1) as f32 * self.spacing,
        );

        let size =
            limits.resolve(Length::Shrink, Length::Shrink, intrinsic_size);

        layout::Node::new(size)
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
        let state = tree.state.downcast_mut::<State>();
        let offset = layout.position() - Point::ORIGIN;

        for (element, (index, layout, tree)) in self
            .visible_elements
            .iter_mut()
            .zip(&mut state.visible_layouts)
        {
            element.as_widget_mut().update(
                tree,
                event,
                Layout::with_offset(
                    offset + Vector::new(0.0, self.spacing * *index as f32),
                    layout,
                ),
                cursor,
                renderer,
                shell,
                viewport,
            );
        }

        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            match &mut state.task {
                Task::Idle => {}
                Task::Computing { .. } => {
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
            }

            let offsets = &state.offsets;

            let start =
                match binary_search_with_index_by(offsets, |i, height| {
                    (*height + i.saturating_sub(1) as f32 * self.spacing)
                        .partial_cmp(&(viewport.y - offset.y))
                        .unwrap_or(Ordering::Equal)
                }) {
                    Ok(i) => i,
                    Err(i) => i.saturating_sub(1),
                }
                .min(self.content.len());

            let end = match binary_search_with_index_by(offsets, |i, height| {
                (*height + i.saturating_sub(1) as f32 * self.spacing)
                    .partial_cmp(&(viewport.y + viewport.height - offset.y))
                    .unwrap_or(Ordering::Equal)
            }) {
                Ok(i) => i,
                Err(i) => i,
            }
            .min(self.content.len());

            if state.visible_outdated
                || state.visible_layouts.len() != self.visible_elements.len()
            {
                self.visible_elements.clear();
                state.visible_outdated = false;
            }

            // If view was recreated, we repopulate the visible elements
            // out of the internal visible layouts
            if self.visible_elements.is_empty() {
                self.visible_elements = state
                    .visible_layouts
                    .iter()
                    .map(|(i, _, _)| {
                        (self.view_item)(*i, &self.content.items[*i])
                    })
                    .collect();
            }

            // Clear no longer visible elements
            let top = state
                .visible_layouts
                .iter()
                .take_while(|(i, _, _)| *i < start)
                .count();

            let bottom = state
                .visible_layouts
                .iter()
                .rev()
                .take_while(|(i, _, _)| *i >= end)
                .count();

            let _ = self.visible_elements.splice(..top, []);
            let _ = state.visible_layouts.splice(..top, []);

            let _ = self
                .visible_elements
                .splice(self.visible_elements.len() - bottom.., []);
            let _ = state
                .visible_layouts
                .splice(state.visible_layouts.len() - bottom.., []);

            // Prepend new visible elements
            if let Some(first_visible) =
                state.visible_layouts.first().map(|(i, _, _)| *i)
                && start < first_visible
            {
                for (i, item) in
                    self.content.items[start..first_visible].iter().enumerate()
                {
                    let mut element = (self.view_item)(start + i, item);
                    let mut tree = Tree::new(&element);
                    element.as_widget_mut().diff(&mut tree);

                    let layout = element
                        .as_widget_mut()
                        .layout(&mut tree, renderer, &state.last_limits)
                        .move_to((
                            0.0,
                            offsets[start + i]
                                + (start + i) as f32 * self.spacing,
                        ));

                    state.visible_layouts.insert(i, (start + i, layout, tree));
                    self.visible_elements.insert(i, element);
                }
            }

            // Append new visible elements
            let last_visible = state
                .visible_layouts
                .last()
                .map(|(i, _, _)| *i + 1)
                .unwrap_or(start);

            if last_visible < end {
                for (i, item) in
                    self.content.items[last_visible..end].iter().enumerate()
                {
                    let mut element = (self.view_item)(last_visible + i, item);
                    let mut tree = Tree::new(&element);
                    element.as_widget_mut().diff(&mut tree);

                    let layout = element
                        .as_widget_mut()
                        .layout(&mut tree, renderer, &state.last_limits)
                        .move_to((
                            0.0,
                            offsets[last_visible + i]
                                + (last_visible + i) as f32 * self.spacing,
                        ));

                    state.visible_layouts.push((
                        last_visible + i,
                        layout,
                        tree,
                    ));
                    self.visible_elements.push(element);
                }
            }

            // Forward RedrawRequested to all visible elements so child
            // widgets (buttons, etc.) can compute their visual status.
            for (element, (index, item_layout, tree)) in self
                .visible_elements
                .iter_mut()
                .zip(&mut state.visible_layouts)
            {
                element.as_widget_mut().update(
                    tree,
                    event,
                    Layout::with_offset(
                        offset + Vector::new(0.0, self.spacing * *index as f32),
                        item_layout,
                    ),
                    cursor,
                    renderer,
                    shell,
                    viewport,
                );
            }
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let offset = layout.position() - Point::ORIGIN;

        for (element, (_item, layout, tree)) in
            self.visible_elements.iter().zip(&state.visible_layouts)
        {
            element.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                Layout::with_offset(offset, layout),
                cursor,
                viewport,
            );
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
        let state = tree.state.downcast_ref::<State>();
        let offset = layout.position() - Point::ORIGIN;

        self.visible_elements
            .iter()
            .zip(&state.visible_layouts)
            .map(|(element, (_item, layout, tree))| {
                element.as_widget().mouse_interaction(
                    tree,
                    Layout::with_offset(offset, layout),
                    cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let offset = layout.position() - Point::ORIGIN;

        for (element, (_item, layout, tree)) in self
            .visible_elements
            .iter_mut()
            .zip(&mut state.visible_layouts)
        {
            element.as_widget_mut().operate(
                tree,
                Layout::with_offset(offset, layout),
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State>();
        let offset = layout.position() - Point::ORIGIN;

        let children = self
            .visible_elements
            .iter_mut()
            .zip(&mut state.visible_layouts)
            .filter_map(|(child, (_item, layout, tree))| {
                child.as_widget_mut().overlay(
                    tree,
                    Layout::with_offset(offset, layout),
                    renderer,
                    _viewport,
                    translation,
                )
            })
            .collect::<Vec<_>>();

        (!children.is_empty())
            .then(|| overlay::Group::with_children(children).overlay())
    }
}

impl<'a, T, Message, Theme, Renderer>
    From<List<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: core::Renderer + 'a,
{
    fn from(list: List<'a, T, Message, Theme, Renderer>) -> Self {
        Self::new(list)
    }
}

/// The backing data for a [`List`].
///
/// Holds the full item vector and tracks mutations (push, remove,
/// update) so the widget can reconcile its tree incrementally
/// instead of rebuilding from scratch every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content<T> {
    items: Vec<T>,
    is_new: RefCell<bool>,
    changes: RefCell<VecDeque<Change>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Updated { original: usize, current: usize },
    Removed { original: usize, current: usize },
    Pushed { original: usize, current: usize },
}

impl<T> Content<T> {
    /// Creates an empty [`Content`].
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            is_new: RefCell::new(true),
            changes: RefCell::new(VecDeque::new()),
        }
    }

    /// Creates a [`Content`] pre-populated with `items`.
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            items,
            is_new: RefCell::new(true),
            changes: RefCell::new(VecDeque::new()),
        }
    }

    /// Returns a reference to the item at `index`, if it exists.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Returns a mutable reference to the item at `index`, recording
    /// the mutation so the widget tree stays in sync.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.changes.borrow_mut().push_back(Change::Updated {
            original: index,
            current: index,
        });
        self.items.get_mut(index)
    }

    /// Appends an item to the end of the list.
    pub fn push(&mut self, item: T) {
        let index = self.items.len();

        self.changes.borrow_mut().push_back(Change::Pushed {
            original: index,
            current: index,
        });

        self.items.push(item);
    }

    /// Removes and returns the item at `index`, updating pending
    /// change indices accordingly.
    pub fn remove(&mut self, index: usize) -> T {
        let mut changes = self.changes.borrow_mut();

        // Update pending changes after removal
        changes.retain_mut(|change| match change {
            Change::Updated { current, .. }
            | Change::Removed { current, .. }
            | Change::Pushed { current, .. }
                if *current > index =>
            {
                // Decrement index of later changes
                *current -= 1;

                true
            }
            _ => true,
        });

        changes.push_back(Change::Removed {
            original: index,
            current: index,
        });

        self.items.remove(index)
    }

    /// Returns `true` if the list contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items in the list.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Consumes the [`Content`] and returns the underlying [`Vec<T>`].
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T> Default for Content<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FromIterator<T> for Content<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::with_items(iter.into_iter().collect())
    }
}

/// SAFETY: Copied from the `std` library.
#[allow(unsafe_code)]
fn binary_search_with_index_by<'a, T, F>(
    slice: &'a [T],
    mut f: F,
) -> Result<usize, usize>
where
    F: FnMut(usize, &'a T) -> Ordering,
{
    use std::cmp::Ordering::*;

    // INVARIANTS:
    // - 0 <= left <= left + size = right <= self.len()
    // - f returns Less for everything in self[..left]
    // - f returns Greater for everything in self[right..]
    let mut size = slice.len();
    let mut left = 0;
    let mut right = size;
    while left < right {
        let mid = left + size / 2;

        // SAFETY: the while condition means `size` is strictly positive, so
        // `size/2 < size`. Thus `left + size/2 < left + size`, which
        // coupled with the `left + size <= self.len()` invariant means
        // we have `left + size/2 < self.len()`, and this is in-bounds.
        let cmp = f(mid, unsafe { slice.get_unchecked(mid) });

        // This control flow produces conditional moves, which results in
        // fewer branches and instructions than if/else or matching on
        // cmp::Ordering.
        // This is x86 asm for u8: https://rust.godbolt.org/z/698eYffTx.
        left = if cmp == Less { mid + 1 } else { left };
        right = if cmp == Greater { mid } else { right };
        if cmp == Equal {
            return Ok(mid);
        }

        size = right - left;
    }

    Err(left)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::widget::tree::Tree;
    use crate::core::{Element, Event, Never, Theme};

    const LIMITS: layout::Limits = layout::Limits::new(
        Size::ZERO,
        Size {
            width: 1024.0,
            height: 768.0,
        },
    );

    fn fixed_item<'a>(_i: usize, _: &'a u8) -> Element<'a, Never, Theme, ()> {
        iced_widget::Space::new().width(100).height(20).into()
    }

    #[test]
    fn three_items_compute_in_one_pass() {
        let content: Content<u8> = Content::with_items(vec![1, 2, 3]);
        let mut list: Element<'_, Never, Theme, ()> =
            List::new(&content, fixed_item).spacing(10).into();

        let mut tree = Tree::new(&list);
        list.as_widget_mut().diff(&mut tree);

        let node = list.as_widget_mut().layout(&mut tree, &(), &LIMITS);
        let state = tree.state.downcast_ref::<State>();

        assert!(
            matches!(state.task, Task::Idle),
            "3 items < batch 50, should finish in one pass"
        );
        assert_eq!(state.offsets.len(), 4, "3 items => 4 offsets");
        assert_eq!(state.offsets, vec![0.0, 20.0, 40.0, 60.0]);
        assert_eq!(state.size, Size::new(100.0, 60.0));

        // width=Shrink resolves to intrinsic (100), height=60 + 2*spacing(10) = 80
        assert_eq!(node.size(), Size::new(100.0, 80.0));
    }

    #[test]
    fn update_with_redraw_populates_visible_elements() {
        let content: Content<u8> = Content::with_items(vec![1, 2, 3]);
        let mut widget = List::new(&content, fixed_item).spacing(10);

        let mut tree = Tree::new(Element::<Never, Theme, ()>::from(
            List::new(&content, fixed_item).spacing(10),
        ));
        let element: Element<'_, Never, Theme, ()> =
            List::new(&content, fixed_item).spacing(10).into();
        tree.diff(element);

        // Layout first to compute offsets
        let layout_node = widget.layout(&mut tree, &(), &LIMITS);

        // Now simulate RedrawRequested
        let redraw = Event::Window(window::Event::RedrawRequested(
            core::time::Instant::now(),
        ));
        let viewport = Rectangle::with_size(Size::new(1024.0, 768.0));
        let waker = core::shell::Waker::noop();
        let window = crate::core::window::Headless;
        let mut messages: Vec<Never> = vec![];
        let mut shell = Shell::new(&window, waker, &mut messages);

        widget.update(
            &mut tree,
            &redraw,
            Layout::new(&layout_node),
            mouse::Cursor::Unavailable,
            &(),
            &mut shell,
            &viewport,
        );

        let state = tree.state.downcast_ref::<State>();
        eprintln!("visible_layouts: {}", state.visible_layouts.len());
        eprintln!("visible_elements: {}", widget.visible_elements.len());

        assert!(
            !widget.visible_elements.is_empty(),
            "After RedrawRequested with computed offsets, visible elements should be populated"
        );
    }
}
