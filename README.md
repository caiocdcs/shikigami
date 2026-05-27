# Shikigami

Self-hosted heartbeat and cron monitor built in Rust.
Single binary. SQLite-backed. Notification support with ntfy, gotify, slack and email.

## Purpose

Inspired by [healthchecks.io](https://healthchecks.io). Uses a dead man's switch approach:
if a ping is not received within the expected time + grace period, a notification is sent.

- Create monitors for jobs/tasks with schedule and grace period config
- Ping API to signal liveness
- Notifications to multiple channels (ntfy, gotify, slack, email)

## API

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Liveness check |
| GET | `/health/ready` | Readiness check (verifies DB) |

### Monitors

| Method | Path | Description |
|--------|------|-------------|
| POST | `/monitors` | Create a monitor |
| PUT | `/monitors/{uuid}` | Update a monitor |
| DELETE | `/monitors/{uuid}` | Delete a monitor |

### Integrations

| Method | Path | Description |
|--------|------|-------------|
| POST | `/integrations` | Create an integration |
| PUT | `/integrations/{uuid}` | Update an integration |
| DELETE | `/integrations/{uuid}` | Delete an integration |

### Ingress

| Method | Path | Description |
|--------|------|-------------|
| POST | `/ping/{uuid}` | Signal monitor is alive |
| POST | `/success/{uuid}` | Report success |
| POST | `/failure/{uuid}` | Report failure |

## Data Model

```
monitors  1--*  check_ins
monitors  *--*  integrations   (via monitor_integrations)
monitors  1--*  notification_outbox
integrations  1--*  notification_outbox
```

| Table | Purpose |
|-------|---------|
| `monitors` | Registered monitors with schedule, grace, status |
| `check_ins` | Ingress records (ping/success/failure) per monitor |
| `integrations` | Notification channels (ntfy, slack, etc.) |
| `monitor_integrations` | Many-to-many link between monitors and integrations |
| `notification_outbox` | Pending notifications to send (decouples detection from delivery) |

## Job Runner (planned)

Background task that periodically checks monitors where `next_expected_at < now()`.
If no ping within `grace_seconds`, marks the monitor as missed and writes to `notification_outbox`.
The outbox pattern decouples "detect missed" from "send notification".

## Project Structure

```
src/
  api/          Routes and handlers
  config.rs     Environment config
  lib.rs        App state, pool, router setup
  main.rs       Entrypoint
migrations/     Sqlx migrations
```

## Configuration

Environment variables (or `.env` file, `__` as separator):

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Listen port |
| `DATABASE_URL` | required | SQLite connection string |
| `LOG_LEVEL` | `info` | Tracing level |

Example `.env`:

```
DATABASE_URL=sqlite:shikigami.db?mode=rwc
```

## Development

```
nix develop                  # enter dev shell
cargo build                  # build
cargo test                   # test
cargo run                    # run
sqlx migrate add -r <name>   # create migration
sqlx migrate revert          # rollback migration
```

## Stack

- Rust / Tokio / Axum / Sqlx / SQLite

## Backlog

- [ ] postgres support
- [ ] cli to register job
- [ ] ui
- [ ] rate limiting
- [ ] api keys
- [ ] save logs and exit codes
- [ ] history/metrics
- [ ] project space

## License

MIT
