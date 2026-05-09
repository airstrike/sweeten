//! Axis, alignment, and justification enums for the flex engine.
//!
//! These types are the public vocabulary consumed by the flex engine and
//! by the `Row` / `Column` widgets that will be added in a later phase.
//! The naming follows CSS Flexbox conventions exactly — `align-items`,
//! `align-self`, `justify-content` — so users coming from CSS get the
//! mapping for free.
//!
//! `iced::Alignment` (`Start` / `Center` / `End`) maps cleanly into
//! [`AlignItems`] and [`AlignSelf`] via `From` impls, so callers can
//! continue to write `Row::align(iced::Center)` for the common case.
//! Stretch — which `iced::Alignment` does not have — is reached via
//! the explicit enum variant.

use crate::core::{Alignment, Size};

/// The main axis of a flex layout.
///
/// A `Row` lays out along [`Axis::Horizontal`]; a `Column` along
/// [`Axis::Vertical`]. The cross axis is always the perpendicular one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Lay out children left-to-right (or right-to-left when reversed).
    Horizontal,
    /// Lay out children top-to-bottom (or bottom-to-top when reversed).
    Vertical,
}

impl Axis {
    /// Returns the main-axis component of a [`Size`].
    pub fn main(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    /// Returns the cross-axis component of a [`Size`].
    pub fn cross(self, size: Size) -> f32 {
        match self {
            Axis::Horizontal => size.height,
            Axis::Vertical => size.width,
        }
    }

    /// Packs a `(main, cross)` pair into the underlying `(x, y)` order
    /// for this [`Axis`].
    pub fn pack<T>(self, main: T, cross: T) -> (T, T) {
        match self {
            Axis::Horizontal => (main, cross),
            Axis::Vertical => (cross, main),
        }
    }
}

/// Container-level cross-axis alignment for every child that does not
/// override it.
///
/// Mirrors CSS `align-items`. `Stretch` has no `iced::Alignment`
/// equivalent — it must be set via this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignItems {
    /// Pack children at the cross-start edge.
    #[default]
    Start,
    /// Pack children at the cross-end edge.
    End,
    /// Center children on the cross axis.
    Center,
    /// Stretch children to fill the container's cross size.
    ///
    /// Honored only by children whose cross length is fluid; children
    /// with a fixed cross length are unaffected.
    Stretch,
}

/// Per-child cross-axis alignment that overrides the container's
/// [`AlignItems`].
///
/// Mirrors CSS `align-self`. The default — [`AlignSelf::Auto`] — defers
/// to the container's [`AlignItems`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignSelf {
    /// Inherit the container's [`AlignItems`].
    #[default]
    Auto,
    /// Pack at the cross-start edge.
    Start,
    /// Pack at the cross-end edge.
    End,
    /// Center on the cross axis.
    Center,
    /// Stretch to fill the container's cross size.
    Stretch,
}

/// Main-axis distribution of children when the container has leftover
/// main-axis space.
///
/// Mirrors CSS `justify-content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Justify {
    /// Pack at the main-start edge.
    #[default]
    Start,
    /// Pack at the main-end edge.
    End,
    /// Pack around the main-axis center.
    Center,
    /// Distribute leftover space equally between adjacent items, with
    /// none at the start or end.
    SpaceBetween,
    /// Distribute leftover space equally around items, so each item has
    /// half-units of padding on each side and adjacent items share a
    /// full unit between them.
    SpaceAround,
    /// Distribute leftover space equally between items and at both
    /// ends, so every gap (including the start- and end-edge gap) is
    /// the same.
    SpaceEvenly,
}

impl From<Alignment> for AlignItems {
    fn from(alignment: Alignment) -> Self {
        match alignment {
            Alignment::Start => AlignItems::Start,
            Alignment::Center => AlignItems::Center,
            Alignment::End => AlignItems::End,
        }
    }
}

impl From<AlignItems> for Alignment {
    /// `Stretch` collapses to `Center`, matching iced's flex resolver
    /// behavior — `iced_core::Alignment` has no stretch variant, so the
    /// closest preserve-the-axis spelling is `Center`.
    fn from(align: AlignItems) -> Self {
        match align {
            AlignItems::Start => Alignment::Start,
            AlignItems::End => Alignment::End,
            AlignItems::Center | AlignItems::Stretch => Alignment::Center,
        }
    }
}

impl From<Alignment> for AlignSelf {
    fn from(alignment: Alignment) -> Self {
        match alignment {
            Alignment::Start => AlignSelf::Start,
            Alignment::Center => AlignSelf::Center,
            Alignment::End => AlignSelf::End,
        }
    }
}

impl From<AlignItems> for AlignSelf {
    fn from(align: AlignItems) -> Self {
        match align {
            AlignItems::Start => AlignSelf::Start,
            AlignItems::End => AlignSelf::End,
            AlignItems::Center => AlignSelf::Center,
            AlignItems::Stretch => AlignSelf::Stretch,
        }
    }
}

impl AlignSelf {
    /// Resolves an [`AlignSelf`] against its container's [`AlignItems`],
    /// returning the [`AlignItems`] that applies to this child.
    ///
    /// `Auto` defers to `container`; every other variant overrides.
    pub fn resolve(self, container: AlignItems) -> AlignItems {
        match self {
            AlignSelf::Auto => container,
            AlignSelf::Start => AlignItems::Start,
            AlignSelf::End => AlignItems::End,
            AlignSelf::Center => AlignItems::Center,
            AlignSelf::Stretch => AlignItems::Stretch,
        }
    }
}
