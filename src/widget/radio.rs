//! Radio buttons let users choose a single option from a set.
//!
//! The top-level [`radio`] function builds a [`Group`]: a focus-managed set
//! of buttons that acts as one tab stop (the WAI-ARIA "radiogroup"), with
//! arrow-key navigation and selection that follows focus. Each button has
//! an animated dot that fades and scales in while the fill and border
//! colors interpolate in unison, instead of snapping the moment the
//! selection flips.
//!
//! For a lone button in a bespoke layout, [`radio::single`](single()) builds
//! a [`Single`] — the classic one-circular-button-per-value widget, a
//! sweetened drop-in for [`iced`'s `radio`].
//!
//! Both share the same styling ([`default`], [`legacy`], [`Style`],
//! [`Status`], [`Catalog`]) and ring/dot rendering.
//!
//! [`iced`'s `radio`]: https://docs.iced.rs/iced/widget/radio/index.html
pub mod content;
mod dot;
pub mod group;
pub mod single;
mod style;

pub use content::Content;
pub use group::Group;
pub use single::Single;
pub use style::{Catalog, Status, Style, StyleFn, default, legacy};

use crate::core::text;

/// Creates a new focus-managed [`Group`] of radio buttons from the current
/// `selected` value, the set of `options`, and a `view` function producing
/// each option's label.
///
/// This is the default `radio`: the whole group is a single tab stop and
/// the arrow keys rove between the enabled options while selection follows
/// focus. The returned [`Group`] is disabled until
/// [`on_select`](Group::on_select) is called; the `selected` value may be
/// `None`, which is the usual state on init. For a lone button, use
/// [`single`].
pub fn radio<'a, V, T, Message, Theme, Renderer>(
    selected: Option<V>,
    options: impl IntoIterator<Item = V>,
    view: impl Fn(&V) -> T,
) -> Group<'a, V, Message, Theme, Renderer>
where
    V: Eq + Clone,
    T: Into<Content<'a, Theme, Renderer>>,
    Theme: Catalog + 'a,
    Renderer: text::Renderer,
{
    Group::new(selected, options, view)
}

/// Creates a new [`Single`] radio button for the given `value`, showing as
/// selected when it matches the current `selected` value.
///
/// This is the escape hatch for a lone button in a bespoke layout; most
/// callers want the focus-managing [`Group`] that [`radio`] builds. The
/// returned [`Single`] is disabled until
/// [`on_toggle`](Single::on_toggle) is called.
pub fn single<'a, V, Message, Theme, Renderer>(
    value: V,
    selected: Option<V>,
) -> Single<'a, V, Message, Theme, Renderer>
where
    V: Eq,
    Theme: Catalog + 'a,
    Renderer: text::Renderer,
{
    Single::new(value, selected)
}
