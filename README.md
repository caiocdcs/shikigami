# Shikigami

[![CI](https://github.com/caiocdcs/shikigami/actions/workflows/ci.yml/badge.svg)](https://github.com/caiocdcs/shikigami/actions/workflows/ci.yml)

Self-hosted heartbeat and cron monitor built in Rust.
Single binary. SQLite-backed. Notifications via ntfy, gotify, email.

Inspired by [healthchecks.io](https://healthchecks.io). It is a dead man's
switch: if a ping is not received within the expected time plus a grace
period, an alert is sent.

- Register monitors with interval or cron schedules
- HTTP ping API for liveness signals
- Background detection of missed monitors
- Notification dispatch with retry (ntfy / gotify / email)
- Outbox pattern decouples detection from delivery

## How it works

Your job pings an HTTP endpoint when it runs. A background checker looks for
monitors whose expected time plus grace has passed without a ping and writes a
failure check-in. A second worker drains a notification outbox and delivers
alerts with retry. See `ARCHITECTURE.md` for internals.

## Install

One-line install (Linux x86_64/aarch64):

```sh
curl -sSfL https://github.com/caiocdcs/shikigami/releases/latest/download/install.sh | sh
```

The script downloads the release tarball and verifies its sha256sum before
installing. Since you are piping a remote script to a shell, you may prefer to
inspect it first:

```sh
curl -sSfL https://github.com/caiocdcs/shikigami/releases/latest/download/install.sh -o install.sh
less install.sh && sh install.sh
```

With Nix:

```sh
nix run github:caiocdcs/shikigami
```

From source:

```sh
cargo build --release
./target/release/shikigami
```

Docker:

```sh
mkdir -p data
docker run -d --name shikigami -p 3000:3000 -v "$PWD/data:/var/lib/shikigami" \
  --restart unless-stopped ghcr.io/caiocdcs/shikigami:latest
```

Or use the example Compose file at [`examples/docker-compose.yml`](examples/docker-compose.yml).

## Configuration

Environment variables (or a `.env` file, `__` as nesting separator):

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | required | SQLite connection string (e.g. `sqlite:shikigami.db?mode=rwc`) |
| `LOG_LEVEL` | `info` | `tracing` filter (e.g. `info,sqlx=warn`) |
| `PORT` | `3000` | Listen port |
| `API_KEY` | unset | Bearer token required for CRUD and `/health/report`. Unset = open + startup warning. |
| `UI_ENABLED` | `false` | Serve public status page at `/status` (no auth). |
| `RETENTION_DAYS` | `30` | Check-ins older than this are pruned. `0` disables retention. |
| `RETENTION_INTERVAL_SECONDS` | `3600` | How often the retention worker runs. |
| `OUTBOX_RETENTION_DAYS` | `30` | Terminal outbox rows (sent/failed) older than this are pruned. `0` disables. |
| `POOL_MAX_CONNECTIONS` | `10` | SQLite pool max connections. |
| `POOL_MIN_CONNECTIONS` | `2` | SQLite pool min connections. |
| `POOL_ACQUIRE_TIMEOUT_SECONDS` | `3` | Pool acquire timeout. |
| `POOL_IDLE_TIMEOUT_SECONDS` | `600` | Pool idle timeout. |
| `NOTIFICATION_INTERVAL_SECONDS` | `30` | How often the notification worker drains the outbox. |
| `NOTIFICATION_MAX_RETRIES` | `3` | Max delivery attempts before an outbox entry is marked failed. |
| `CHECKER_INTERVAL_SECONDS` | `60` | How often the missed-monitor checker runs. |

Example `.env`:

```
PORT=3000
DATABASE_URL=sqlite:shikigami.db?mode=rwc
LOG_LEVEL=info
```

> **Security note:** The SQLite database contains notification credentials
> (SMTP passwords, gotify tokens) in plaintext. Treat the database file with
> the same care as `.env` -- restrict filesystem access to the shikigami user
> only. The Docker image runs as non-root (UID 1000) and the default volume
> path (`/var/lib/shikigami`) follows this practice.

## Run

```sh
DATABASE_URL=sqlite:shikigami.db?mode=rwc shikigami
```

Migrations run automatically on first start. The binary is self-contained.

## Deploy

Pick a deployment target; all three keep the database at `/var/lib/shikigami`.
The binary is self-contained (SQLite is bundled), so no system `libsqlite3` is
required on any deployment.

- [Docker / Docker Compose](docs/deployment/docker.md) - multi-arch image on `ghcr.io`, single container.
- [systemd](docs/deployment/systemd.md) - bare-metal/VM with the prebuilt binary.
- [NixOS](docs/deployment/nixos.md) - flake module (`services.shikigami`), builds from source.

## API

When `API_KEY` is set, all management endpoints (`/monitors*`, `/integrations*`,
`/health/report`) require `Authorization: Bearer <key>`. The ingress endpoints
(`/ping`, `/success`, `/failure`) and health probes (`/health`, `/health/ready`)
stay open so monitored jobs can ping without a key.

### Status UI

When `UI_ENABLED=true`, a read-only status page is available:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/status` | Status overview: summary counts + table of all monitors |
| GET | `/status/{slug}` | Monitor detail: config + last 20 check-ins |

The UI is public (no auth) and reuses the same data as `/health/report`.
Designed for airgapped/self-hosted use — no external CSS/JS.

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

Cron monitors accept an optional `timezone` (IANA name, e.g. `America/Sao_Paulo`),
so `0 9 * * *` fires at 9am local time. It defaults to `UTC` and is ignored for
interval monitors. Timestamps are always stored and returned in UTC.

### Integrations

| Method | Path | Description |
|--------|------|-------------|
| POST | `/integrations` | Create an integration |
| GET | `/integrations` | List integrations |
| GET | `/integrations/{id}` | Get an integration |
| PUT | `/integrations/{id}` | Update an integration |
| DELETE | `/integrations/{id}` | Delete an integration |

Config JSON for each channel is documented in [`docs/integrations.md`](docs/integrations.md).

### Ingress (called by monitored jobs)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/ping/{id}` | Heartbeat (creates check-in with outcome=success) |
| POST | `/success/{id}` | Explicit success report |
| POST | `/failure/{id}` | Failure report (triggers notification) |

`{id}` is either the monitor UUID or its slug, so `POST /ping/my-job` works as well as
`POST /ping/<uuid>`. Slugs are restricted to `[A-Za-z0-9_-]` (length 1-50).

All three ingress endpoints accept an optional raw-text body stored as the
check-in `message`. Empty body = no message (backward compatible). Capped at
16 KiB (413 on overflow). For failures, the message is added to the notification
body as `Reason: <message>` (truncated to 256 chars).

```sh
# Failure with a reason:
backup.sh 2>&1 | curl -X POST --data-binary @- http://localhost:3000/failure/my-job
```

## Example: nightly backup monitor

```bash
# 1. Create the monitor (daily at 03:00, with 1-hour grace)
MON_ID=$(curl -s -X POST http://localhost:3000/monitors \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"name":"nightly-backup","slug":"nightly-backup","schedule_type":"cron","cron_expr":"0 3 * * *","timezone":"America/Sao_Paulo","grace_seconds":3600}' \
  | jq -r .id)

# 2. Create a notification integration (ntfy)
INT_ID=$(curl -s -X POST http://localhost:3000/integrations \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $API_KEY" \
  -d '{"name":"alerts","channel":"ntfy","config":{"url":"https://ntfy.sh","topic":"homelab","priority":5,"message":"alert"}}' \
  | jq -r .id)

# 3. Link them
curl -X POST http://localhost:3000/monitors/$MON_ID/integrations \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $API_KEY" \
  -d "{\"integration_id\":\"$INT_ID\"}"

# 4. In your backup script
0 3 * * * /usr/local/bin/backup.sh && curl -X POST http://localhost:3000/ping/$MON_ID
```

If the backup script fails to run or fails to ping, after `grace_seconds`
the background checker detects the miss and a notification is dispatched.

## Development

```sh
just            # lint, build, test
just test       # run tests
just lint       # clippy with strict warnings
just ci         # fmt-check, lint, test, audit
just migrate    # run migrations
just sqlx-prepare  # regenerate offline sqlx query cache
```

See `CONTRIBUTING.md` and `AGENTS.md` before opening a pull request.

## License

[MIT](LICENSE)
