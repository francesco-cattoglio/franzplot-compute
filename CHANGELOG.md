# Changelog
All notable changes to this project will be documented in this file.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

## [21.4.0] - 2020-04-06:

### Added
- this changelog
- you can now drag & drop files on top of Franzplot to open them.
- all input expressions are now analyzed, and more meaningful errors will be produced.
- added the "Sample parameter" node.
- added more default camera views for the scene tab. They are bound to the `5`, `6` and `7` keys.
- scene camera view should feel much more smooth under Windows.

### Changed
- to enter an exponentiation, use the `^` operator, i.e. `x^2`. The old `pow(x, n)` function is no longer available.
- the "quality" slider can now go down to 1. Setting quality to 1 can help Windows user running on software rasterization (WARP).

### Fixed
- trying to apply a parametric transform to a primitive will no longer crash the application.
- expressions containing divisions between integers (i.e: 1/2 instead of 1.0/2.0) will now produce the correct result.
