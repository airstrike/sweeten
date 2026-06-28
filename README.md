<div align="center">

<img src="assets/logo.svg" width=400>

## `sweeten` your daily `iced` brew

[![Crates.io](https://img.shields.io/crates/v/sweeten.svg)](https://crates.io/crates/sweeten)
[![Documentation](https://docs.rs/sweeten/badge.svg)](https://docs.rs/sweeten)
[![License](https://img.shields.io/crates/l/sweeten.svg)](https://github.com/airstrike/sweeten/blob/master/LICENSE)
[![Made with iced](https://iced.rs/badge.svg)](https://github.com/iced-rs/iced)

</div>

## Overview

`sweeten` provides drop-in replacements for common `iced` widgets with extra
features for more complex use cases. Each widget is designed to feel like the
upstream version with extra capabilities layered on top.

## Installation

If you're using the latest `iced` release:

```bash
cargo add sweeten
```

If you're tracking `iced` from git, add this to your `Cargo.toml`:

```toml
sweeten = { git = "https://github.com/airstrike/sweeten", branch = "master" }
```

## Widgets

### `Button`

Adds focus and blur methods:

- `.on_focus(Message)` fires when the button gains keyboard focus
- `.on_blur(Message)` fires when it loses focus

Pairs with `sweeten::widget::operation::{focus_next, focus_previous}` to build
keyboard-navigable forms across any focusable widget.

### `Toggler`

Animates state changes. The handle slides between positions and the fill
color crossfades between the off and on styles.

```rust
toggler(self.is_on)
    .label("Enable notifications")
    .on_toggle(Message::Toggled)
```

### `Checkbox`

Animated checkmark with fade and scale on toggle, and a crossfaded fill and
border.

```rust
checkbox(self.is_checked)
    .label("Subscribe to updates")
    .on_toggle(Message::Toggled)
```

The press and release gesture matches native checkboxes: pressing inside arms
the checkbox and releasing inside fires the toggle. Pressing outside and
dragging in, or pressing inside and dragging out before release, cancels.

Five built-in styles ship: `primary`, `secondary`, `success`, `danger`, and
`text` (a monochrome variant that uses the theme's body text color for the
fill, pairing naturally with text-only buttons). Each variant uses
`<swatch>.base` for the active state, `<swatch>.strong` for hovered, and
fades toward the page background when disabled.

### `ProgressBar`

Animates between values. Each render whose `value` differs from the
currently-displayed value eases toward the new target over 150ms, so stepping
the bar through discrete checkpoints reveals a smooth fill instead of jumps.

```rust
progress_bar(0.0..=100.0, self.progress)
    .girth(4.0)
    .on_idle(Message::LoadingSettled)
```

The optional `.on_idle(f32)` fires once the easing animation settles
at its target, making it useful for gating follow-up actions (dismissing a
splash, navigating, etc.) on the bar reaching a specific value.

### `MouseArea`

Adds `on_press_with` for capturing the click position via a closure:

```rust
mouse_area("Click me and I'll tell you where!",)
    .on_press_with(|point| Message::ClickWithPoint(point)),
```

### `PickList`

Accepts an optional closure to disable some items:

```rust
pick_list(
    &Language::ALL[..],
    Some(|languages: &[Language]| {
        languages
            .iter()
            .map(|lang| matches!(lang, Language::Javascript))
            .collect()
    }),
    self.selected_language,
    Message::Pick,
)
.placeholder("Choose a language...");
```

> Note that the compiler cannot currently infer the type of the closure, so
> you may need to specify it explicitly as shown above.

### `TextInput`

Adds focus-related features:

- `.on_focus` and `.on_blur` methods for handling focus events
- Sweetened `focus_next` and `focus_previous` focus management functions,
  which return the ID of the focused element

### `Row` and `Column`

Adds drag-and-drop reordering of children via `.on_drag`:

```rust
use sweeten::widget::column;
use sweeten::widget::drag::DragEvent;

column(items.iter().map(|s| s.as_str().into()))
    .spacing(5)
    .on_drag(Message::Reorder)
    .into()
```

### `Table`

Column headers are optional: pass `None` to `table::column()` for a
headerless column, and when every column is headerless the header row is
skipped entirely. Additional features on top of upstream:

- `.sticky_header(true)` pins the header row when scrolling inside a
  parent scrollable
- `.header_underline_height(px)` draws a distinct separator below the
  header (independent thickness and color from regular row separators)
- `.border(px)` draws an outline around the entire table

```rust
use sweeten::widget::table::{self, table};

table(
    [
        table::column(Some(text("Name").into()), |r: Row| text(r.name).into()),
        table::column(None, |r: Row| text(r.age).into()),
    ],
    rows,
)
.sticky_header(true)
.header_underline_height(2)
.border(1)
```

### `List`

A virtualized list that only materializes visible items, ported from iced's
`feat/list-widget-redux` branch. Suitable for large or unbounded data sets
where creating a widget per row would be too expensive.

```rust
use sweeten::widget::list;

let content = list::Content::from_iter(
    (0..10_000).map(|id| format!("Item {id}"))
);

list(&content, |_index, label| text(label).into())
    .spacing(5)
```

### `FitText`

A text widget that auto-scales its font size to fit the bounds it is laid
out into. Like CSS' `clamp(min, ideal, max)`, but the "ideal" is solved for
instead of specified: `sweeten` binary-searches the size range and picks the
largest font that still fits.

```rust
use iced::Fill;
use sweeten::widget::fit_text;

fit_text("Big headline")
    .max_size(120)
    .min_size(16)
    .width(Fill)
    .height(Fill)
    .center()
```

Both `min_size` and `max_size` are optional; call neither and the font
scales within `[1.0, 1024.0]` pixels by default.

### `Transition`

A single-slot container that animates a slide whenever its child value
changes. The new content slides into the canonical position from the edge
opposite the configured `Direction`, while the previous content slides off
the same-side edge.

```rust
use sweeten::widget::transition::{self, Direction};

transition::transition(self.phrase.clone(), |s: &String| {
    text(s.clone()).size(22).into()
})
.direction(Direction::Up)
.into()
```

`Direction` is sugar for the more general `Mode` knob
(`.mode(Mode::Slide(d))`).

## Examples

See [`examples/`](examples/) for the full list, or run one directly:

```bash
cargo run --example mouse_area
cargo run --example pick_list
cargo run --example text_input
cargo run --example fit_text
cargo run --example list
cargo run --example transition
cargo run --example table
cargo run --example checkbox
cargo run --example progress_bar
```

## Contributing

Contributions are welcome. Fork the repository, create a feature branch,
implement your changes with tests, and submit a PR.

## License

MIT

## Acknowledgements

- [iced](https://github.com/iced-rs/iced)
- [Rust programming language](https://www.rust-lang.org/)
