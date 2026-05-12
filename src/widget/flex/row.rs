//! A horizontal flex container — CSS-flex `Row`.
//!
//! Distributes its children along the main (horizontal) axis using the
//! CSS Flexbox algorithm. Plain elements enter as [`FlexChild`] values
//! with CSS-default properties; users opt into per-item flex behaviour
//! by wrapping a child in `flex::flex(elem)` and chaining
//! [`FlexChild::grow`], [`FlexChild::shrink`], [`FlexChild::basis`], or
//! [`FlexChild::align_self`].
//!
//! See the [module-level documentation][super] for a tour of the API.
//!
//! [`FlexChild::grow`]: super::FlexChild::grow
//! [`FlexChild::shrink`]: super::FlexChild::shrink
//! [`FlexChild::basis`]: super::FlexChild::basis
//! [`FlexChild::align_self`]: super::FlexChild::align_self

use crate::core::layout::{self, Layout};
use crate::core::mouse;
use crate::core::overlay;
use crate::core::renderer;
use crate::core::widget::{Operation, Tree};
use crate::core::{
    Element, Event, Length, Padding, Pixels, Rectangle, Shell, Size, Vector,
    Widget,
};

use super::alignment::{AlignItems, Axis, Justify};
use super::child::FlexChild;
use super::engine;

/// A horizontal flex container.
///
/// Mirrors CSS `display: flex; flex-direction: row` semantics — the
/// main axis is horizontal, the cross axis is vertical, and each child
/// can carry its own `flex-grow`, `flex-shrink`, `flex-basis`, and
/// `align-self` via [`FlexChild`].
///
/// See the [module-level documentation][super] for usage examples and
/// the full builder API.
#[allow(missing_debug_implementations)]
pub struct Row<'a, Message, Theme = crate::Theme, Renderer = crate::Renderer> {
    spacing: f32,
    padding: Padding,
    width: Length,
    height: Length,
    align: AlignItems,
    justify: Justify,
    reverse: bool,
    clip: bool,
    children: Vec<FlexChild<'a, Message, Theme, Renderer>>,
}

impl<'a, Message, Theme, Renderer> Row<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    /// Creates an empty [`Row`].
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    /// Creates a [`Row`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_vec(Vec::with_capacity(capacity))
    }

    /// Creates a [`Row`] from an iterator of plain [`Element`] children.
    ///
    /// Each element enters as a [`FlexChild`] with CSS-default flex
    /// properties.
    pub fn with_children(
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self::with_flex_children(children.into_iter().map(FlexChild::from))
    }

    /// Creates a [`Row`] from an iterator of [`FlexChild`] children.
    pub fn with_flex_children(
        children: impl IntoIterator<Item = FlexChild<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let iterator = children.into_iter();
        Self::from_vec(Vec::with_capacity(iterator.size_hint().0))
            .extend_flex(iterator)
    }

    /// Creates a [`Row`] from an already allocated [`Vec`] of
    /// [`FlexChild`].
    pub fn from_vec(
        children: Vec<FlexChild<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            spacing: 0.0,
            padding: Padding::ZERO,
            width: Length::Shrink,
            height: Length::Shrink,
            align: AlignItems::Start,
            justify: Justify::Start,
            reverse: false,
            clip: false,
            children,
        }
    }

    /// Sets the spacing between adjacent items along the main axis.
    pub fn spacing(mut self, amount: impl Into<Pixels>) -> Self {
        self.spacing = amount.into().0;
        self
    }

    /// Sets the spacing between adjacent items along the main axis.
    ///
    /// Alias of [`Row::spacing`] using the CSS `gap` name.
    pub fn gap(self, amount: impl Into<Pixels>) -> Self {
        self.spacing(amount)
    }

    /// Sets the [`Padding`] of the [`Row`].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the width of the [`Row`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Row`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the cross-axis alignment for every child that does not
    /// override it via [`FlexChild::align_self`].
    ///
    /// Accepts both the explicit [`AlignItems`] enum and `iced`'s
    /// canonical [`Alignment`] consts (`iced::Start`, `iced::Center`,
    /// `iced::End`) via the `Into<AlignItems>` impl.
    ///
    /// [`Alignment`]: crate::core::Alignment
    pub fn align(mut self, align: impl Into<AlignItems>) -> Self {
        self.align = align.into();
        self
    }

    /// Sets the main-axis distribution mode (CSS `justify-content`).
    pub fn justify(mut self, justify: impl Into<Justify>) -> Self {
        self.justify = justify.into();
        self
    }

    /// Reverses the visual main-axis order.
    ///
    /// Mirrors CSS `flex-direction: row-reverse`. The first source
    /// child appears at the visual right edge, the last at the left.
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Sets whether the contents of the [`Row`] should be clipped on
    /// overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Pushes a plain [`Element`] onto the [`Row`].
    ///
    /// The element enters as a [`FlexChild`] with CSS-default flex
    /// properties.
    pub fn push(
        self,
        child: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.push_flex(FlexChild::new(child))
    }

    /// Pushes an explicit [`FlexChild`] onto the [`Row`].
    pub fn push_flex(
        mut self,
        child: FlexChild<'a, Message, Theme, Renderer>,
    ) -> Self {
        self.children.push(child);
        self
    }

    /// Pushes a plain element onto the [`Row`] when `child` is
    /// [`Some`], leaving the row unchanged otherwise.
    pub fn push_maybe(
        self,
        child: Option<impl Into<Element<'a, Message, Theme, Renderer>>>,
    ) -> Self {
        match child {
            Some(c) => self.push(c),
            None => self,
        }
    }

    /// Pushes a [`FlexChild`] onto the [`Row`] when `child` is
    /// [`Some`], leaving the row unchanged otherwise.
    pub fn push_maybe_flex(
        self,
        child: Option<FlexChild<'a, Message, Theme, Renderer>>,
    ) -> Self {
        match child {
            Some(c) => self.push_flex(c),
            None => self,
        }
    }

    /// Extends the [`Row`] with the given plain element children.
    pub fn extend(
        self,
        children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        children.into_iter().fold(self, Self::push)
    }

    /// Extends the [`Row`] with the given [`FlexChild`] children.
    pub fn extend_flex(
        self,
        children: impl IntoIterator<Item = FlexChild<'a, Message, Theme, Renderer>>,
    ) -> Self {
        children.into_iter().fold(self, Self::push_flex)
    }
}

impl<Message, Theme, Renderer> Default for Row<'_, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message, Theme, Renderer>
    FromIterator<FlexChild<'a, Message, Theme, Renderer>>
    for Row<'a, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    fn from_iter<
        T: IntoIterator<Item = FlexChild<'a, Message, Theme, Renderer>>,
    >(
        iter: T,
    ) -> Self {
        Self::with_flex_children(iter)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Row<'_, Message, Theme, Renderer>
where
    Renderer: crate::core::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        self.children
            .iter()
            .map(|c| Tree::new(c.content().as_widget()))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children_custom(
            &self.children,
            |state, child| state.diff(child.content().as_widget()),
            |child| Tree::new(child.content().as_widget()),
        );
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
        // Materialise the per-child Properties up front. The Vec is
        // owned, so passing `&props` to `engine::resolve` does not
        // borrow `self.children` — the layouter closure below is then
        // free to mutably borrow each child by index. Kept as `Vec`
        // (not `SmallVec`) because this is a one-shot allocation per
        // layout call — `engine::resolve` consumes it as `&[_]`, so
        // the inline-vs-heap discriminator on the SmallVec access
        // path would be pure overhead for any non-trivial N.
        let props: Vec<_> = self
            .children
            .iter()
            .map(|c| c.resolved_properties(Axis::Horizontal))
            .collect();

        // Split-borrow: take disjoint mutable slices of children and
        // tree-children outside the closure so the closure does not
        // re-borrow `self` or `tree` as a whole on every invocation.
        let children = self.children.as_mut_slice();
        let trees = tree.children.as_mut_slice();

        engine::resolve(
            Axis::Horizontal,
            limits,
            self.width,
            self.height,
            self.padding,
            self.spacing,
            self.justify,
            self.align,
            self.reverse,
            &props,
            |idx, item_limits| {
                children[idx].content_mut().as_widget_mut().layout(
                    &mut trees[idx],
                    renderer,
                    item_limits,
                )
            },
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .content_mut()
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
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
        for ((child, state), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.content_mut().as_widget_mut().update(
                state, event, layout, cursor, renderer, shell, viewport,
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
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, state), layout)| {
                child.content().as_widget().mouse_interaction(
                    state, layout, cursor, viewport, renderer,
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
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            let viewport = if self.clip {
                &clipped_viewport
            } else {
                viewport
            };

            for ((child, state), layout) in self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .filter(|(_, layout)| layout.bounds().intersects(viewport))
            {
                child.content().as_widget().draw(
                    state, renderer, theme, style, layout, cursor, viewport,
                );
            }
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
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((child, state), layout)| {
                child.content_mut().as_widget_mut().overlay(
                    state,
                    layout,
                    renderer,
                    viewport,
                    translation,
                )
            })
            .collect::<Vec<_>>();

        (!children.is_empty())
            .then(|| overlay::Group::with_children(children).overlay())
    }
}

impl<'a, Message, Theme, Renderer> From<Row<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: crate::core::Renderer + 'a,
{
    fn from(row: Row<'a, Message, Theme, Renderer>) -> Self {
        Element::new(row)
    }
}

/// Creates a flex [`Row`] from a comma-separated list of children.
///
/// Each `expr` is converted into a [`FlexChild`] via the
/// `From<E: Into<Element>>` blanket impl, so plain elements and
/// `flex(elem).grow(...)` calls can be mixed freely.
///
/// `#[macro_export]` lands this macro at the consuming crate's root
/// (e.g. `sweeten::flex_row!`); the canonical path is
/// `sweeten::widget::flex::row!` via the in-module re-alias just below
/// this macro.
#[macro_export]
macro_rules! flex_row {
    () => (
        $crate::widget::flex::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::widget::flex::Row::with_flex_children(
            [$($crate::widget::flex::FlexChild::from($x)),+]
        )
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::flex::flex;
    use iced_core::Theme;
    use iced_widget::{Renderer, button, text};

    /// Compile-only smoke test: the macro accepts a mix of plain
    /// elements and `flex(elem).grow(...)` calls against iced's real
    /// `Theme` and `Renderer`. Exercises every entry-point spelling —
    /// `crate::flex_row!`, `flex::row!`, `flex::row(...)`, and direct
    /// `Row::new()` — so the three-namespace re-alias at
    /// `widget::flex::row` is covered. Layout correctness is the
    /// engine tests' job.
    #[test]
    fn _compiles() {
        // `crate::flex_row!` — `#[macro_export]` form at the crate root.
        let _root_macro: Row<'_, (), Theme, Renderer> = crate::flex_row![
            text("a"),
            flex(button("b").on_press(())).grow(1.0),
            flex(text("c")).basis(120.0),
        ]
        .gap(8)
        .padding(12)
        .align(AlignItems::Center)
        .justify(Justify::SpaceBetween)
        .reverse(false);

        // `flex::row!` — re-aliased macro at the canonical module path.
        let _aliased_macro: Row<'_, (), Theme, Renderer> =
            crate::widget::flex::row![text("x"), flex(text("y")).shrink(0.0)];

        // `flex::row(...)` — free function constructor.
        let _from_fn: Row<'_, (), Theme, Renderer> =
            crate::widget::flex::row([text("a").into(), text("b").into()]);

        // Empty macro variant.
        let _empty: Row<'_, (), Theme, Renderer> = crate::flex_row![];

        // Direct constructor + push_maybe_flex / push_maybe variants.
        let none_child: Option<Element<'_, (), Theme, Renderer>> = None;
        let _direct: Row<'_, (), Theme, Renderer> = Row::new()
            .push(text("z"))
            .push_flex(flex(text("w")).grow(2.0))
            .push_maybe(none_child)
            .push_maybe(Some(text("opt")));
    }
}
