//! Theming shared by the single [`Single`](super::single::Single) radio
//! button and the [`Group`](super::group::Group).
use crate::core::theme::palette;
use crate::core::{Background, Color, Theme};

/// The possible status of a radio button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The radio button can be interacted with.
    Active {
        /// Indicates whether the radio button is currently selected.
        is_selected: bool,
    },
    /// The radio button is being hovered.
    Hovered {
        /// Indicates whether the radio button is currently selected.
        is_selected: bool,
    },
    /// The radio button cannot be interacted with.
    Disabled {
        /// Indicates whether the radio button is currently selected.
        is_selected: bool,
    },
}

impl Status {
    /// Returns this [`Status`] with its `is_selected` field replaced.
    pub(super) fn with_selected(self, is_selected: bool) -> Self {
        match self {
            Status::Active { .. } => Status::Active { is_selected },
            Status::Hovered { .. } => Status::Hovered { is_selected },
            Status::Disabled { .. } => Status::Disabled { is_selected },
        }
    }
}

/// The appearance of a radio button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Background`] of the radio button.
    pub background: Background,
    /// The [`Color`] of the dot of the radio button.
    pub dot_color: Color,
    /// The border width of the radio button.
    pub border_width: f32,
    /// The border [`Color`] of the radio button.
    pub border_color: Color,
    /// The text [`Color`] of the radio button.
    pub text_color: Option<Color>,
}

/// The theme catalog of a radio button.
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class with the given status.
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

/// A styling function for a radio button.
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

/// The default style of a radio button.
///
/// Toned down to match the rest of sweeten: an unselected button is a
/// hollow ring with a neutral border; selecting it fills the circle with
/// the primary accent and pops a small contrasting dot at its center —
/// the shadcn/radix "radio" look, and the direct analog of how the
/// [`checkbox`](mod@crate::widget::checkbox) fills on check. Hovering raises
/// the background one step instead of flooding it with color. For the
/// classic iced styling, see [`legacy`].
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    match status {
        Status::Active { is_selected } => styled(
            palette.background.strong.color,
            palette.background.weak.color,
            palette.primary.strong,
            is_selected,
        ),
        Status::Hovered { is_selected } => styled(
            palette.background.strong.color,
            palette.background.strong.color,
            palette.primary.strong,
            is_selected,
        ),
        Status::Disabled { is_selected } => {
            let muted = palette::Pair {
                color: palette
                    .primary
                    .strong
                    .color
                    .mix(palette.background.base.color, 0.6),
                text: palette.primary.strong.text,
            };
            styled(
                palette.background.weak.color,
                palette.background.weaker.color,
                muted,
                is_selected,
            )
        }
    }
}

/// The classic iced style of a radio button: a transparent circle with a
/// primary-colored border and dot; hovering tints the fill with the
/// primary weak color.
pub fn legacy(theme: &Theme, status: Status) -> Style {
    let palette = theme.palette();

    let active = Style {
        background: Color::TRANSPARENT.into(),
        dot_color: palette.primary.strong.color,
        border_width: 1.0,
        border_color: palette.primary.strong.color,
        text_color: None,
    };

    match status {
        Status::Active { .. } => active,
        Status::Hovered { .. } => Style {
            dot_color: palette.primary.strong.color,
            background: palette.primary.weak.color.into(),
            ..active
        },
        Status::Disabled { .. } => {
            // Mute the accent toward the page background so a disabled
            // radio reads as inert while still hinting its selection.
            let muted = palette
                .primary
                .strong
                .color
                .mix(palette.background.base.color, 0.6);
            Style {
                dot_color: muted,
                border_color: muted,
                ..active
            }
        }
    }
}

/// Builds a [`Style`] from a neutral `border_color`, an unselected fill
/// `base`, and the `accent` [`Pair`](palette::Pair) used when selected.
///
/// Unselected renders as a hollow ring over `base`; selected fills the
/// circle with `accent.color` and draws the center dot in `accent.text`
/// so it reads against the fill. The dot color stays `accent.text` in
/// both states so it doesn't shift hue as the dot fades in.
fn styled(
    border_color: Color,
    base: Color,
    accent: palette::Pair,
    is_selected: bool,
) -> Style {
    let (background, border_color) = if is_selected {
        (accent.color, accent.color)
    } else {
        (base, border_color)
    };

    Style {
        background: Background::Color(background),
        dot_color: accent.text,
        border_width: 1.0,
        border_color,
        text_color: None,
    }
}
