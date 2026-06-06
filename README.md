# Shikigami

Self-hosted heartbeat and cron monitor built in Rust.
Single binary. SQLite-backed. Notifications via ntfy, gotify, slack.

## Install

One-line install (Linux x86_64/aarch64, macOS x86_64/Apple Silicon):

```sh
curl -sSfL https://github.com/caiocesaralves/shikigami/releases/latest/download/install.sh | sh
```

Or with Nix:

```sh
nix build github:caiocdcs/shikigami
./result/bin/shikigami
```

Or build from source:

```sh
cargo build --release
./target/release/shikigami
```

## Purpose

Inspired by [healthchecks.io](https://healthchecks.io). Dead man's switch:
if a ping is not received within the expected time + grace period, an alert is sent.

- Register monitors with interval or cron schedules
- HTTP ping API for liveness signals
- Background detection of missed monitors
- Notification dispatch with retry (ntfy / gotify / slack)
- Outbox pattern decouples detection from delivery

## API

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness check |
| GET | `/health/ready` | Readiness check (verifies DB) |
| GET | `/health/report` | Report of all monitors with status, integrations, pending outbox |

### Monitors

| Method | Path | Description |
|--------|------|-------------|
| POST | `/monitors` | Create a monitor |
| GET | `/monitors` | List monitors |
| GET | `/monitors/{id}` | Get a monitor |
| PUT | `/monitors/{id}` | Update a monitor |
| DELETE | `/monitors/{id}` | Delete a monitor |
| GET | `/monitors/{id}/check-ins` | History of check-ins (latest 50) |
| GET | `/monitors/{id}/integrations` | Linked notification integrations |
| POST | `/monitors/{id}/integrations` | Link an integration to a monitor |
| DELETE | `/monitors/{id}/integrations/{integration_id}` | Unlink an integration |

### Integrations

| Method | Path | Description |
|--------|------|-------------|
| POST | `/integrations` | Create an integration |
| GET | `/integrations` | List integrations |
| GET | `/integrations/{id}` | Get an integration |
| PUT | `/integrations/{id}` | Update an integration |
| DELETE | `/integrations/{id}` | Delete an integration |

### Ingress (called by monitored jobs)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/ping/{uuid}` | Heartbeat (creates check-in with outcome=success) |
| POST | `/success/{uuid}` | Explicit success report |
| POST | `/failure/{uuid}` | Failure report (triggers notification) |

## Example: nightly backup monitor

```bash
# 1. Create the monitor (daily at 03:00, with 1-hour grace)
MON_ID=$(curl -s -X POST http://localhost:3000/monitors \
  -H 'Content-Type: application/json' \
  -d '{"name":"nightly-backup","slug":"nightly-backup","schedule_type":"cron","cron_expr":"0 3 * * *","grace_seconds":3600}' \
  | jq -r .id)

# 2. Create a notification integration (ntfy)
INT_ID=$(curl -s -X POST http://localhost:3000/integrations \
  -H 'Content-Type: application/json' \
  -d '{"name":"alerts","channel":"ntfy","config":{"url":"https://ntfy.sh","topic":"homelab","priority":5,"message":"alert"}}' \
  | jq -r .id)

# 3. Link them
curl -X POST http://localhost:3000/monitors/$MON_ID/integrations \
  -H 'Content-Type: application/json' \
  -d "{\"integration_id\":\"$INT_ID\"}"

# 4. In your backup script
0 3 * * * /usr/local/bin/backup.sh && curl -X POST http://localhost:3000/ping/$MON_ID
```

If the backup script fails to run or fails to ping, after `grace_seconds`
the background checker detects the miss and a notification is dispatched.

## Architecture

```
api/      Routes and handlers
core/     Business logic
  domain/   Entities and value types
  ports/    Trait definitions for repositories and dispatchers
  *.rs      Services and background workers
spi/      Service Provider Implementations
  *_repository.rs    SQLite repositories
  *_dispatcher.rs    HTTP notification clients
migrations/  SQL migrations
```

Hexagonal: core depends only on ports. SPI implements ports. Composition in `lib.rs`.

## Data Model

```
monitors  1--*  check_ins
monitors  *--*  integrations  (via monitor_integrations)
monitors  1--*  notification_outbox
integrations  1--*  notification_outbox
```

| Table | Purpose |
|-------|---------|
| `monitors` | Registered monitors with schedule, grace, status |
| `check_ins` | History of pings (success/failure) |
| `integrations` | Notification channels and their configs |
| `monitor_integrations` | Many-to-many link |
| `notification_outbox` | Pending notifications: pending -> sending -> sent / failed |

## Background workers

Two `tokio` tasks run alongside the HTTP server:

- **Notification dispatcher** (every 30s): polls `notification_outbox`,
  dispatches via HTTP to ntfy/gotify/slack, retries transient errors up to 3 times
- **Missed-monitor checker** (every 60s): finds monitors where
  `next_expected_at + grace_seconds < now()` and creates a failure check-in

Both shut down gracefully on SIGTERM/Ctrl+C.

## Configuration

Environment variables (or `.env`, `__` as separator):

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | required | SQLite connection string (e.g. `sqlite:shikigami.db?mode=rwc`) |
| `LOG_LEVEL` | `info` | `tracing` filter (e.g. `info,sqlx=warn`) |
| `PORT` | `3000` | Listen port |

Example `.env`:

```
PORT=3000
DATABASE_URL=sqlite:shikigami.db?mode=rwc
LOG_LEVEL=info
```

## Run

```sh
DATABASE_URL=sqlite:shikigami.db?mode=rwc shikigami
```

Migrations run automatically on first start. The binary is self-contained.

## Development

```
cargo build                          # build
cargo test                           # 35 tests
cargo run                            # run (uses .env)
sqlx migrate add -r <name>           # create migration
sqlx migrate revert --database-url sqlite:shikigami.db?mode=rwc
```

To regenerate the offline sqlx query cache after schema/query changes:

```
DATABASE_URL=sqlite:shikigami.db?mode=rwc cargo sqlx prepare
```

## Stack

Rust 2024 / Tokio / Axum / sqlx / SQLite / reqwest

## Backlog

- [ ] email dispatcher (SMTP via `lettre`)
- [ ] retention policy for `check_ins`
- [ ] API key authentication
- [ ] rate limiting
- [ ] cli for monitor registration
- [ ] web UI
- [ ] PostgreSQL support
- [ ] metrics endpoint
- [ ] systemd unit and Dockerfile examples

## License

MIT
