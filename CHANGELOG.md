# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com), and this project adheres to [Semantic Versioning](https://semver.org).

## [Unreleased]

### Added

- Ingress endpoints (`/ping`, `/success`, `/failure`) accept an optional raw-text body
  stored as the check-in `message`. For failures it is added to the notification body
  as `Reason: <message>` (truncated to 256 chars). 16 KiB body cap on ingress routes
  (413 on overflow).
- Check-in retention worker: prunes `check_ins` rows older than `RETENTION_DAYS`
  (default 30) every `RETENTION_INTERVAL_SECONDS` (default 3600). Bounds SQLite
  growth for long-running homelab instances. Set `RETENTION_DAYS=0` to disable.
- Operational tuning via env vars: `POOL_MAX_CONNECTIONS`, `POOL_MIN_CONNECTIONS`,
  `POOL_ACQUIRE_TIMEOUT_SECONDS`, `POOL_IDLE_TIMEOUT_SECONDS`,
  `NOTIFICATION_INTERVAL_SECONDS`, `NOTIFICATION_MAX_RETRIES`,
  `CHECKER_INTERVAL_SECONDS`. Defaults match prior hardcoded values.

### Changed

- `check_ins.comments` column renamed to `message` (migration
  `20260619120000_rename_check_ins_comments_to_message`). Breaking; runs on startup.
- `CheckInResponse.comments` JSON field renamed to `message`.

## [0.4.1] - 2026-06-13

### Fixed

- Sqlx offline metadata regenerated for paginated check-ins query
- Release workflow: use CHANGELOG.md content instead of auto-generated notes
- Release workflow: use env vars instead of template expressions to prevent
  shell injection via crafted tag names

## [0.4.0] - 2026-06-13

### Added

- `NotificationContent` domain struct: notification messages now carry monitor name,
  slug, and last-seen time instead of raw UUIDs. Each dispatcher (ntfy, gotify, slack)
  uses the title and body fields appropriate to its channel.
- Paginated check-ins endpoint: `GET /monitors/{id}/check-ins?limit=20&offset=0`.
  Returns `CheckInsPage` with `items`, `total`, `limit`, `offset`. Default limit=20, max 100.

### Changed

- Notification message composition moved from `SqliteMonitorRepository` (SPI) to
  `MonitorService` (core). Repos only persist; services compute business values.
- `NotificationDispatcher` trait signature: `message: &str` replaced by
  `notification: &NotificationContent`. Implementations must use the struct fields.
- `OutboxEntry.message: String` replaced by `OutboxEntry.notification: NotificationContent`.
  The `message` column now stores serialized JSON.
- `MonitorRepository::check_in` gains `notification: Option<NotificationContent>` parameter.
- ntfy dispatcher: `Title` header now uses `notification.title` (monitor name) instead of
  hardcoded "Shikigami Alert".
- gotify dispatcher: `title` field uses `notification.title` instead of
  hardcoded "Shikigami Alert".
- slack dispatcher: message heading uses `notification.title` instead of
  hardcoded "Shikigami Alert".
- Status UI monitor detail page reduced to 10 check-ins (was 20).

## [0.3.0] - 2026-06-11

### Added

- API key authentication: set `API_KEY` env var to require `Authorization: Bearer <key>` on
  all management endpoints (`/monitors*`, `/integrations*`, `/health/report`). Ingress
  endpoints (`/ping`, `/success`, `/failure`) and health probes stay open. Unset = CRUD
  open with a startup warning.
- Read-only status UI: set `UI_ENABLED=true` to serve a public status page at `/status`
  with a summary view of all monitors and a detail page at `/status/{slug}` with check-in
  history. Feature-gated, off by default.
- `status_report()` query: single SQL query with aggregated counts (integrations, pending
  outbox) for the status UI and JSON report — no N+1 queries.

### Changed

- `/health/report` JSON endpoint now uses the shared `status_report()` query, fixing the
  N+1 SQL-in-API-layer smell.

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

[Unreleased]: https://github.com/caiocdcs/shikigami/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/caiocdcs/shikigami/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/caiocdcs/shikigami/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/caiocdcs/shikigami/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/caiocdcs/shikigami/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/caiocdcs/shikigami/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/caiocdcs/shikigami/releases/tag/v0.1.0
