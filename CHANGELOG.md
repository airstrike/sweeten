# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Keyboard interaction for `pick_list`: arrow keys, Home/End, and typeahead
  move the highlighted option, Enter selects it, Escape closes the menu,
  and the menu scrolls to keep the highlighted option visible.
- `pick_list::PickList::on_option_hovered` to produce a message when an
  option is highlighted via pointer or keyboard.
- Focus support for `pick_list`, mirroring the sweetened `button`: the
  pick list participates in tab navigation (`PickList::id` targets it),
  opens with Enter, Space, or the arrow keys while focused, closes if
  focus moves away, keeps focus through Escape and selection, and emits
  `PickList::on_focus` / `PickList::on_blur`. The `Status` gained a
  `Focused` variant, drawn as a thicker neutral border by default and a
  primary one in `pick_list::legacy`.
- Titled groups of options in the `pick_list` menu: `pick_list::Options`
  is a list of `pick_list::Group`s built with `pick_list::group` and the
  `options!` macro, spaced via `Options::spacing` (8 by default, matching
  shadcn's group padding) and optionally separated by a rule via
  `PickList::separator`. Plain text titles draw slightly
  smaller and muted; `Element` titles display as-is.
- Inline per-item disabling via `pick_list::option(...).disabled()`.
- Arbitrary widgets as `pick_list` option content: the view function may
  return a `String` or any message-less `Element` (see
  `pick_list::Content`), with `PickList::typeahead` providing the
  matching text for element options. The placeholder is `Content` too.
- Clearing the selection: `pick_list::deselect("None")` — or a literal
  `None` in `options!` — adds a native-menu style "None" entry that
  produces the `PickList::on_deselect` message, shows the check indicator
  while nothing is selected, and is disabled if no handler is set.
- Check indicator next to the selected option in the `pick_list` menu,
  enabled by default and configurable via `PickList::check_indicator`.
- `PickList::menu_width` to override the menu width. By default the menu
  fits its widest entry — including element options the trigger cannot
  measure — and never gets narrower than the pick list, like shadcn's
  content with its trigger-width minimum.
- `PickList::menu_padding` insetting the menu contents from its border,
  defaulting to `4` on every side.
- The `pick_list` menu scrolls without a scrollbar: when the options
  overflow, chevron strips at the top and bottom edges take over — shown
  only in the directions that can scroll, scrolling one row every 50ms
  while hovered, exactly like the scroll buttons of Radix's Select. The
  mouse wheel and keyboard navigation keep scrolling as before, and
  keyboard navigation leaves 4px of breathing room around the row it
  scrolls into view.
- `PickList::anchor` with `pick_list::Anchor`: by default the menu
  overlays the pick list with the selected option aligned on top of it,
  like native menus on macOS and Radix's default `item-aligned` position,
  falling back to `Anchor::Auto` (below, flipping above when cramped)
  when nothing is selected; `Anchor::Top` and `Anchor::Bottom` force a
  side.
- `PickList::radius` rounds the whole control with one knob: the pick
  list border, the menu border, and the menu highlights, which derive
  their radius from it reduced by 2 — shadcn's radius step between a
  menu and its items; group separators span the full menu width,
  ignoring the padding.
- `fit_text` widget that auto-scales font size to fit its bounds. [#13](https://github.com/airstrike/sweeten/pull/13)
- `transition` widget that animates a slide between values when its data changes.
- Animated `checkbox` widget. [#16](https://github.com/airstrike/sweeten/pull/16)
- Self-animating `progress_bar` widget. [#17](https://github.com/airstrike/sweeten/pull/17)
- `table` widget with optional column headers, sticky header, header underline, and table border.
- `list` widget for virtualized lists (only visible items are materialized).
  Modified from hecrj's `List` widget on iced's `feat/list-widget-reloaded`
  branch. [#19](https://github.com/airstrike/sweeten/pull/19)
- README sections for `button`, `toggler`, and `transition`.

### Changed
- `PickList::new` now takes `impl Into<pick_list::Options<T>>` instead of
  `impl Borrow<[T]>` — plain lists still convert directly — and its view
  function may return any `impl Into<pick_list::Content>` instead of only
  `String`.
- `PickList::disabled` now takes a per-option predicate `Fn(&T) -> bool`
  instead of `Fn(&[T]) -> Vec<bool>`.
- `overlay::menu::Menu::new` now takes `pick_list::Options` and a view
  function producing `pick_list::Content`; the menu renders group titles,
  dividers, and element rows, and `menu::Style` gained `label_text_color`
  and `separator_color`. The hovered option defaults to a neutral wash
  (`background.strong`) instead of the primary color, communicating
  selection through the check indicator instead; the pick list border
  likewise stays neutral while hovered or open, raising the background a
  step instead, and the handle draws muted. The classic iced looks
  remain available as `pick_list::legacy` and `menu::legacy` style
  functions.
- Crossfaded `toggler` fill colors during toggle animation.

### Fixed
- Dragged item rendering below items shifted past it during `column` and `row` reorder. [#11](https://github.com/airstrike/sweeten/pull/11)
- Drag reorder canceling when the cursor enters a layer above, such as a `scrollable`, in `column` and `row`. [#15](https://github.com/airstrike/sweeten/pull/15)
- Nested `if` in the `pick_list` wheel-scroll match arm.

## [0.14.0] - 2026-03-02
### Added
- `button` widget with focus and blur support.
- `column` and `row` widgets with drag-and-drop reordering and position tracking.

### Changed
- Tracked released `iced` 0.14.
- Reorganized the module layout to mirror `iced_widget`.

## [0.13.0] - 2025-11-28
### Added
- `text_input` widget with focus and blur messages.

## [0.1.0] - 2024-10-26
### Added
- `mouse_area` widget.
- `pick_list` widget with support for disabled items. [#1](https://github.com/airstrike/sweeten/pull/1)

[Unreleased]: https://github.com/airstrike/sweeten/compare/0.14.0...HEAD
[0.14.0]: https://github.com/airstrike/sweeten/compare/0.13.0...0.14.0
[0.13.0]: https://github.com/airstrike/sweeten/compare/0.1.0...0.13.0
[0.1.0]: https://github.com/airstrike/sweeten/releases/tag/0.1.0
