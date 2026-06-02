// Generated automatically by iced_lucide at build time.
// Do not edit manually.
// 51768eb85c05499fea887dd757d781723f7dd77c91072058cc9fa514f9b6bcda
use iced::widget::text::{self, Text};

pub const FONT: &[u8] = include_bytes!("../fonts/lucide.ttf");

/// All icons as `(name, codepoint_str)` pairs.
/// Use this to populate an icon-picker widget.
#[allow(dead_code)]
pub const ALL_ICONS: &[(&str, &str)] = &[
    ("check", "\u{E06C}"),
    ("grip", "\u{E3B1}"),
    ("lock", "\u{E10B}"),
    ("lock_open", "\u{E10C}"),
    ("move_diagonal_2", "\u{E1C5}"),
    ("pencil", "\u{E1F9}"),
    ("pin", "\u{E259}"),
    ("pin_off", "\u{E2B6}"),
    ("plus", "\u{E13D}"),
    ("square_pen", "\u{E172}"),
    ("trash", "\u{E18E}"),
    ("x", "\u{E1B2}"),
];

pub fn check<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E06C}")
}

pub fn grip<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E3B1}")
}

pub fn lock<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E10B}")
}

pub fn lock_open<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E10C}")
}

pub fn move_diagonal_2<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E1C5}")
}

pub fn pencil<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E1F9}")
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

pub fn plus<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E13D}")
}

pub fn square_pen<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E172}")
}

pub fn trash<'a, Theme>() -> Text<'a, Theme>
where
    Theme: text::Catalog + 'a,
{
    icon("\u{E18E}")
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
