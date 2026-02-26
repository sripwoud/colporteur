# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
