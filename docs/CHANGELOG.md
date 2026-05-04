# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `FitText` widget that auto-scales font size to fit its bounds via binary search. [#13](https://github.com/airstrike/sweeten/pull/13)
- `Transition` widget — single-slot container that animates a slide between values when its `T` changes (Compose `AnimatedContent` / Android `ViewSwitcher` shape). Built around a `Mode` dispatch so future styles (crossfade, fade, wipe, …) drop in as variants.
- README sections for `Button`, `Toggler`, and `Transition`.

### Changed
- Crossfade `Toggler` fill colors during toggle animation instead of snapping.

### Fixed
- `pick_list`: collapse a nested `if` inside the wheel-scroll match arm into a guard (Clippy hygiene; no behavior change).
