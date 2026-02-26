# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/sripwoud/colporteur/releases/tag/v0.1.0) - 2026-02-26

### Added

- *(cli)* add init subcommand and improve missing-config error
- *(cli)* add help descriptions to commands and arguments
- *(cli)* add clap argument parsing and wire main entry point
- *(fetch)* add orchestration pipeline
- *(feed)* generate and manage atom feed files
- *(state)* add json state persistence with atomic writes
- *(sanitize)* add html sanitization and tracking pixel removal
- *(email)* parse raw emails into structured content
- *(config)* add toml config loading and validation
- *(imap)* implement imap client wrapper

### Other

- add release-plz workflow for automated releases
- restructure readme to match auberge/dublette format
- update readme with usage section and add docsify-cli tool
- add docsify documentation site
- *(ci)* rename to master branch, add nextest test job
- replace cargo test with nextest
- update hook config
- add sources/outputs to mise tasks for caching
- update mise tasks
- *(fetch)* add integration tests with mock email source
- *(imap)* extract EmailSource trait for testability
- add rust tooling to mise and dprint
- initialize cargo project with dependencies
- add redacted newsletter eml samples as test fixtures
- Initial commit
