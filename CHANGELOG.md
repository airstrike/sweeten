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
