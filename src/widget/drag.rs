//! Drag-and-drop support for [`Row`] and [`Column`] widgets.
//!
//! This module provides types for handling drag-and-drop reordering of items
//! within [`Row`] and [`Column`] containers.
//!
//! [`Row`]: super::Row
//! [`Column`]: super::Column

/// Events emitted during drag operations.
#[derive(Debug, Clone)]
pub enum DragEvent {
    /// An item was picked up and drag started.
    Picked {
        /// Index of the picked item.
        index: usize,
    },
    /// An item was dropped onto a target position.
    Dropped {
        /// Original index of the dragged item.
        index: usize,
        /// Index of the target position.
        target_index: usize,
    },
    /// The drag was canceled (e.g., cursor left the widget area).
    Canceled {
        /// Index of the item that was being dragged.
        index: usize,
    },
}
