//! Radio buttons let users choose a single option from a set.
//!
//! [`Single`] is a sweetened drop-in for [`iced`'s `radio`] — one circular
//! button per value, with an animated selection dot that fades and scales
//! in while the fill and border colors interpolate in unison, instead of
//! snapping the moment the selection flips.
//!
//! Its styling ([`default`], [`legacy`], [`Style`], [`Status`],
//! [`Catalog`]) and the ring/dot rendering are shared machinery that the
//! other radio widgets in this module build on.
//!
//! [`iced`'s `radio`]: https://docs.iced.rs/iced/widget/radio/index.html
mod dot;
pub mod single;
mod style;

pub use single::Single;
pub use style::{Catalog, Status, Style, StyleFn, default, legacy};
