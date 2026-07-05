//! The label content a [`Group`](super::group::Group) draws next to each
//! radio button.
use std::convert::Infallible;

use crate::core::Element;
use crate::core::text;

/// The content a [`Group`](super::group::Group) displays as the label of an
/// option.
///
/// The view function given to [`Group::new`](super::group::Group::new) can
/// return anything that converts into [`Content`]: a `String` or `&str` for
/// plain text, or an [`Element`] for an arbitrary rich label.
pub enum Content<'a, Theme = crate::Theme, Renderer = crate::Renderer> {
    /// Plain text, drawn with the text settings of the
    /// [`Group`](super::group::Group).
    Text(text::Fragment<'a>),
    /// An arbitrary message-less widget.
    ///
    /// Content is display-only — it is laid out and drawn, but never
    /// receives events — so its elements carry [`Infallible`] as their
    /// message and can never produce one.
    Element(Element<'a, Infallible, Theme, Renderer>),
}

impl<'a, Theme, Renderer> From<String> for Content<'a, Theme, Renderer> {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl<'a, Theme, Renderer> From<&'a str> for Content<'a, Theme, Renderer> {
    fn from(text: &'a str) -> Self {
        Self::Text(text.into())
    }
}

impl<'a, Theme, Renderer> From<Element<'a, Infallible, Theme, Renderer>>
    for Content<'a, Theme, Renderer>
{
    fn from(element: Element<'a, Infallible, Theme, Renderer>) -> Self {
        Self::Element(element)
    }
}
