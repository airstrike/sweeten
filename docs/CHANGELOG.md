# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `FitText` widget that auto-scales its font size to fit its laid-out
  bounds. Think CSS `clamp(min, ideal, max)`, but with the "ideal" solved
  for via binary search. Both `min_size` and `max_size` are optional —
  unset, the font scales within `[1.0, 1024.0]` pixels by default. See
  `examples/fit_text.rs` for a live demo.
