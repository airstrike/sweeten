# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `widget::gt::Table::animate_on_load` — opt-in entrance animation
  that reveals each non-sticky row in place via a height-clip mask
  with a per-row stagger. Matches framer-motion's default `motion.tr`
  keyframe transition (`cubic-bezier(0.25, 0.1, 0.35, 1)` over 300ms
  per row, 50ms between rows; default curve sourced from
  `motion-dom/src/animation/utils/default-transitions.ts`). Sticky
  block (title / subtitle / units caption / column labels) doesn't
  animate; row and column separators stay drawn at full extent so
  rows materialize within a static lattice — clip-mask reveal is the
  iced-native equivalent of motion.tr's fade since iced has no
  generic widget alpha. Animation fires once per widget instance.
- `widget::gt::Table` — declarative grammar-of-tables widget. Selector-based
  styling (`tab_style(target, style)`), typed cells (`Cell::Number` / `Text` /
  `Empty`), pluggable number formatters that take `&Cell` (so empty rendering
  is the formatter's job), and layout layers (title / subtitle / units caption
  / column labels / stub / body / row groups / summary rows / grand summary /
  source notes). Sticky header pins the title block + column labels together.
  `outer_padding_x` insets the first/last column's text (and spanned-row
  content) so the table can sit inside a card whose title is itself inset
  from the card edge — borders and fills still run edge-to-edge.
  Spanner columns, sorting, click-source cells, and per-cell padding overrides
  are out of scope for v1. The existing `widget::table` stays as the terse
  choice for flat data grids.
- `FitText` widget that auto-scales font size to fit its bounds via binary search. [#13](https://github.com/airstrike/sweeten/pull/13)
- `Transition` widget — single-slot container that animates between values when its `T` changes (Compose `AnimatedContent` / Android `ViewSwitcher` shape). Built around a `Mode` dispatch (`Mode::Slide(Direction)` today; `Crossfade`/`Fade`/`Wipe`/`Hero` slot in as additional variants without touching the widget pipeline). `.direction(d)` stays as sugar for the common slide case.
- README sections for `Button`, `Toggler`, and `Transition`.
- `Table::header_underline_height` and `Style::header_underline` to draw a distinct underline below the header row, replacing the standard separator at that boundary. Both default to the existing `separator_y` thickness/color so behavior is unchanged unless opted into.
- `Table::style` and `Table::class` setters for customizing table appearance per-instance, mirroring the pattern used by sweetened `Button` and `PickList`.

### Changed
- Crossfade `Toggler` fill colors during toggle animation instead of snapping.
