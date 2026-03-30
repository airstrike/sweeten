// Generated automatically by iced_lucide at build time.
// Do not edit manually.
// f9ffb474ec0c9a87f18f05912db84d540b4c5eeff58f26a1194d6ab928e0e400
use iced::widget::text::{self, Text};

pub const FONT: &[u8] = include_bytes!("../fonts/lucide.ttf");

/// All icons as `(name, codepoint_str)` pairs.
/// Use this to populate an icon-picker widget.
#[allow(dead_code)]
pub const ALL_ICONS: &[(&str, &str)] = &[
    ("grip", "\u{E3B1}"),
    ("pin", "\u{E259}"),
    ("pin_off", "\u{E2B6}"),
    ("x", "\u{E1B2}"),
];

pub fn grip<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E3B1}")
}

pub fn pin<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E259}")
}

pub fn pin_off<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E2B6}")
}

pub fn x<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E1B2}")
}

/// Render any Lucide icon by its codepoint string.
/// Use this together with [`ALL_ICONS`] to display icons dynamically:
/// ```ignore
/// for (name, cp) in ALL_ICONS {
///     button(render(cp)).on_press(Msg::Pick(name.to_string()))
/// }
/// ```
pub fn render<'a, Theme>(codepoint: &'a str) -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    Text::new(codepoint).font("lucide")
}

fn icon<'a, Theme>(codepoint: &'a str) -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    render(codepoint)
}
