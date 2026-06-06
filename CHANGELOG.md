# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

## [Unreleased]

### Fixed

- Cascade deletes: deleting a monitor now removes check-ins, integration links, and notification outbox entries (FK enforcement + migration)
- New monitors get initial `next_expected_at` computed from their schedule, making them trackable by the missed-monitor checker immediately
- Notification tests now use FK-on pragma matching production behavior

### Changed

- Clippy strict mode: `-D warnings -D clippy::unwrap_used`, no `.unwrap()`/`.expect()` in production code
- Justfile: added `fmt-check`, `audit`, `ci`, `lint-ci`, `sqlx-prepare`, `migrate`, `clean`, `clean-all`
- `cargo-audit` added to nix flake dev shell

### Added

- `.clippy.toml` with MSRV configuration
- `AGENTS.md` with project navigation, architecture, testing, and agent contribution policy
- `CONTRIBUTING.md` with build/test/ci commands and PR checklist
- `CHANGELOG.md`

## [0.1.1] - 2026-05-25

### Added

- `GET /health/report` endpoint for operational overview

## [0.1.0] - 2026-05-24

### Added

- Monitor CRUD (interval and cron schedules)
- Integration CRUD (ntfy, gotify, email, slack)
- Monitor-integration linking
- Ingress endpoints: `POST /ping/{id}`, `POST /success/{id}`, `POST /failure/{id}`
- Missed-monitor detection and automatic failure check-in
- Notification dispatch (ntfy, gotify, slack)
- SQLite with foreign key enforcement

[Unreleased]: https://github.com/caiocdcs/shikigami/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/caiocdcs/shikigami/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/caiocdcs/shikigami/releases/tag/v0.1.0
