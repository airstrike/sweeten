//! Position tracking for widget items.
//!
//! This module provides the ability to track the layout positions of items
//! within [`Row`](crate::widget::Row) and [`Column`](crate::widget::Column)
//! widgets, enabling features like scroll-to-item.
//!
//! # Example
//!
//! ```no_run
//! use sweeten::widget::operation::position;
//!
//! # #[derive(Debug, Clone)]
//! # enum Message { Found(Option<iced_core::Rectangle>) }
//! # fn update(column_id: &position::Id) -> iced_runtime::Task<Message> {
//! // Find the bounds of item 5 inside the given widget:
//! position::find_position(column_id.clone(), 5).map(Message::Found)
//! # }
//! ```

use std::any::Any;
use std::collections::HashMap;

use crate::core::Rectangle;
use crate::core::widget;
use crate::core::widget::Operation;
use crate::core::widget::operation::Outcome;

/// An identifier for a positioned widget.
///
/// Wraps a [`widget::Id`] so that [`Row`](crate::widget::Row) and
/// [`Column`](crate::widget::Column) can expose per-item positions
/// through the widget operation system.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id(pub(crate) widget::Id);

impl Id {
    /// Creates a new [`Id`] from the given value.
    pub fn new(id: impl Into<widget::Id>) -> Self {
        Self(id.into())
    }

    /// Creates a unique [`Id`].
    pub fn unique() -> Self {
        Self(widget::Id::unique())
    }
}

impl From<Id> for widget::Id {
    fn from(id: Id) -> Self {
        id.0
    }
}

/// A trait for tracking positions of items within a widget.
pub trait Position {
    /// Stores the bounds of the item at the given index.
    fn set(&mut self, index: usize, bounds: Rectangle);

    /// Returns the bounds of the item at the given index, if available.
    fn get(&self, index: usize) -> Option<Rectangle>;

    /// Clears all stored positions.
    fn clear(&mut self);
}

/// Widget state that tracks item positions.
///
/// Stored as part of a widget's tree state and exposed via
/// [`Operation::custom`] for position queries.
pub struct State(Box<dyn Position>);

impl Default for State {
    fn default() -> Self {
        Self(Box::new(PositionMap::default()))
    }
}

impl State {
    /// Returns the bounds of the item at the given index.
    pub fn get(&self, index: usize) -> Option<Rectangle> {
        self.0.get(index)
    }

    /// Stores the bounds of the item at the given index.
    pub fn set(&mut self, index: usize, bounds: Rectangle) {
        self.0.set(index, bounds);
    }

    /// Clears all stored positions.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Default)]
struct PositionMap(HashMap<usize, Rectangle>);

impl Position for PositionMap {
    fn set(&mut self, index: usize, bounds: Rectangle) {
        self.0.insert(index, bounds);
    }

    fn get(&self, index: usize) -> Option<Rectangle> {
        self.0.get(&index).copied()
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

/// Produces a [`Task`](iced_runtime::Task) that finds the position of the item
/// at the given `index` within the widget identified by `target`.
///
/// Returns `Some(Rectangle)` with the item's layout bounds if found,
/// or `None` if the index is out of range.
///
/// The task produces no value if the target widget was not found.
pub fn find_position(
    target: Id,
    index: usize,
) -> iced_runtime::Task<Option<Rectangle>> {
    iced_runtime::task::widget(FindPosition {
        target,
        index,
        result: None,
    })
}

struct FindPosition {
    target: Id,
    index: usize,
    result: Option<Option<Rectangle>>,
}

impl Operation<Option<Rectangle>> for FindPosition {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn Operation<Option<Rectangle>>),
    ) {
        if self.result.is_none() {
            operate(self);
        }
    }

    fn custom(
        &mut self,
        id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Any,
    ) {
        if self.result.is_none()
            && let Some(id) = id
            && *id == self.target.0
            && let Some(positions) = state.downcast_ref::<State>()
        {
            self.result = Some(positions.get(self.index));
        }
    }

    fn finish(&self) -> Outcome<Option<Rectangle>> {
        match self.result {
            Some(result) => Outcome::Some(result),
            None => Outcome::None,
        }
    }
}
