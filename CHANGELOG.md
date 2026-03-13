# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/sripwoud/colporteur/compare/v0.4.0...v0.5.0) - 2026-03-13

### Added

- add `url` and `base_url` for atom entry links ([#32](https://github.com/sripwoud/colporteur/pull/32))

## [0.4.0](https://github.com/sripwoud/colporteur/compare/v0.3.4...v0.4.0) - 2026-03-13

### Added

- add `export-opml` command ([#29](https://github.com/sripwoud/colporteur/pull/29))

## [0.3.4](https://github.com/sripwoud/colporteur/compare/v0.3.3...v0.3.4) - 2026-03-12

### Added

- support command-based password resolution ([#26](https://github.com/sripwoud/colporteur/pull/26))

## [0.3.3](https://github.com/sripwoud/colporteur/compare/v0.3.2...v0.3.3) - 2026-03-12

### Fixed

- *(imap)* clamp UID search range start to 1 ([#24](https://github.com/sripwoud/colporteur/pull/24))
- downgrade IMAP search with no results from ERROR to DEBUG ([#22](https://github.com/sripwoud/colporteur/pull/22))

## [0.3.2](https://github.com/sripwoud/colporteur/compare/v0.3.1...v0.3.2) - 2026-03-12

### Fixed

- use FHS-compliant path in sample config ([#18](https://github.com/sripwoud/colporteur/pull/18))

## [0.3.1](https://github.com/sripwoud/colporteur/compare/v0.3.0...v0.3.1) - 2026-02-27

### Added

- *(ci)* add sha256 checksums to release binaries

## [0.3.0](https://github.com/sripwoud/colporteur/compare/v0.2.6...v0.3.0) - 2026-02-27

### Added

- [**breaking**] replace password_env with inline password field ([#15](https://github.com/sripwoud/colporteur/pull/15))

## [0.2.6](https://github.com/sripwoud/colporteur/compare/v0.2.5...v0.2.6) - 2026-02-26

### Fixed

- *(ci)* make vendored openssl conditional on non-windows targets

## [0.2.5](https://github.com/sripwoud/colporteur/compare/v0.2.4...v0.2.5) - 2026-02-26

### Fixed

- *(ci)* drop x86_64-apple-darwin target (macos intel runners deprecated)

## [0.2.4](https://github.com/sripwoud/colporteur/compare/v0.2.3...v0.2.4) - 2026-02-26

### Fixed

- *(ci)* replace deprecated macos-13 runner with macos-15-large
- *(ci)* use correct release-plz output field for tag extraction

## [0.2.3](https://github.com/sripwoud/colporteur/compare/v0.2.2...v0.2.3) - 2026-02-26

### Fixed

- *(ci)* vendor openssl for aarch64-linux cross-compilation

## [0.2.2](https://github.com/sripwoud/colporteur/compare/v0.2.1...v0.2.2) - 2026-02-26

### Fixed

- *(ci)* pass release tag to binary upload action

## [0.2.1](https://github.com/sripwoud/colporteur/compare/v0.2.0...v0.2.1) - 2026-02-26

### Fixed

- *(ci)* add cross-toolchain setup for aarch64-linux builds

### Other

- add pre-compiled binary install option ([#7](https://github.com/sripwoud/colporteur/pull/7))
- replace broken release trigger with workflow_dispatch for manual binary builds
- add multi-platform binary release to release workflow ([#6](https://github.com/sripwoud/colporteur/pull/6))

## [0.2.0](https://github.com/sripwoud/colporteur/compare/v0.1.0...v0.2.0) - 2026-02-26

### Added

- _(cli)_ add scan command to discover sender addresses ([#2](https://github.com/sripwoud/colporteur/pull/2))

### Other

- publish pre-compiled binaries on GitHub release ([#3](https://github.com/sripwoud/colporteur/pull/3))
