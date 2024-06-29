# Change Log

Notable changes to this project should be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

<!-- next-header -->
## [Unreleased] - ReleaseDate

## [0.1.4] - 2024-06-29

### Added

 * `--from-rev` and `--to-rev` are now accepted on the command line to
   set the range of changes to look at when using a VCS
 * Get changed files from git

## [0.1.3] - 2024-06-22

### Added

 * `builtin/debug-print-environment` command to show the environment the
   started processes run in
 * Pass more information to the processes being run in the environment:
   * BEAUTYTIPS_INPUTS: Where the input file list came from
   * BEAUTYTIPS_VCS: The VCS being used (if INPUTS is `vcs`)
   * BEAUTYTIPS_VCS_FROM_REV and BEAUTYTIPS_VCS_TO_REV: The revision range
     to compare (empty if default)

### Fixed

 * Input filters got some tests and fixes
 * The `github/check_actions` fixed to not run always

## [0.1.2] - 2024-06-16

* Add windows support

## [0.1.1] - 2024-06-16

## Fixed

* user config file is now ignored when not present

## Internal

* Do not try to build windows binaries on release: Those do not work
* Test MacOS in CI

## [0.1.0] - 2024-06-15

Initial Release

<!-- next-url -->
[Unreleased]: https://github.com/hunger/beautytips/compare/v0.1.4...HEAD
[0.1.4]: https://github.com/hunger/beautytips/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/hunger/beautytips/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/hunger/beautytips/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/hunger/beautytips/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/hunger/beautytips/compare/45bd7663096c68181152f84e11a881a6111e5549...v0.1.0
