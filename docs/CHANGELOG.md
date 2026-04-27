# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `FitText` widget that auto-scales font size to fit its bounds via binary search. [#13](https://github.com/airstrike/sweeten/pull/13)
- README sections for `Button` and `Toggler`.
- `Table::header_underline_height` and `Style::header_underline` to draw a distinct underline below the header row, replacing the standard separator at that boundary. Both default to the existing `separator_y` thickness/color so behavior is unchanged unless opted into.
- `Table::style` and `Table::class` setters for customizing table appearance per-instance, mirroring the pattern used by sweetened `Button` and `PickList`.

### Changed
- Crossfade `Toggler` fill colors during toggle animation instead of snapping.
