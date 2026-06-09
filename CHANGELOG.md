# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.2.0] - 2026-06-09

### Added

- Slug-based pings: ingress endpoints (`/ping`, `/success`, `/failure`) accept either a monitor UUID or its slug, so `POST /ping/my-job` works. Backward compatible with UUID pings.
- Per-monitor `timezone` for cron schedules: cron expressions are evaluated in the configured IANA timezone (defaults to `UTC`), so `0 9 * * *` fires at 9am local time. Timestamps are still stored and returned in UTC. Exposed in monitor and health-report responses.
- `.clippy.toml` with MSRV configuration
- `AGENTS.md` with project navigation, architecture, testing, and agent contribution policy
- `CONTRIBUTING.md` with build/test/ci commands and PR checklist
- `CHANGELOG.md`

### Changed

- Slug validation hardened to URL-safe characters (`[A-Za-z0-9_-]`, length 1-50) on create/update so slugs are valid path segments
- Clippy strict mode: `-D warnings -D clippy::unwrap_used`, no `.unwrap()`/`.expect()` in production code
- Justfile: added `fmt-check`, `audit`, `ci`, `lint-ci`, `sqlx-prepare`, `migrate`, `clean`, `clean-all`
- `cargo-audit` added to nix flake dev shell

### Fixed

- `DELETE /monitors/{id}` now returns 404 for an unknown id instead of 204
- Release workflow now uploads `install.sh` as a release asset, so the README one-line installer (`releases/latest/download/install.sh`) resolves
- Cascade deletes: deleting a monitor now removes check-ins, integration links, and notification outbox entries (FK enforcement + migration)
- New monitors get initial `next_expected_at` computed from their schedule, making them trackable by the missed-monitor checker immediately
- Notification tests now use FK-on pragma matching production behavior

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

[Unreleased]: https://github.com/caiocdcs/shikigami/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/caiocdcs/shikigami/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/caiocdcs/shikigami/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/caiocdcs/shikigami/releases/tag/v0.1.0
