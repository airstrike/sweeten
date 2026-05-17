# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `fit_text` widget that auto-scales font size to fit its bounds. [#13](https://github.com/airstrike/sweeten/pull/13)
- `transition` widget that animates a slide between values when its data changes.
- Animated `checkbox` widget. [#16](https://github.com/airstrike/sweeten/pull/16)
- Self-animating `progress_bar` widget. [#17](https://github.com/airstrike/sweeten/pull/17)
- README sections for `button`, `toggler`, and `transition`.

### Changed
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
