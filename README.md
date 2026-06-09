# Shikigami

[![CI](https://github.com/caiocdcs/shikigami/actions/workflows/ci.yml/badge.svg)](https://github.com/caiocdcs/shikigami/actions/workflows/ci.yml)

Self-hosted heartbeat and cron monitor built in Rust.
Single binary. SQLite-backed. Notifications via ntfy, gotify, slack.

Inspired by [healthchecks.io](https://healthchecks.io). It is a dead man's
switch: if a ping is not received within the expected time plus a grace
period, an alert is sent.

- Register monitors with interval or cron schedules
- HTTP ping API for liveness signals
- Background detection of missed monitors
- Notification dispatch with retry (ntfy / gotify / slack)
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

Inspect the script before piping it to a shell if you prefer:

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

## Configuration

Environment variables (or a `.env` file, `__` as nesting separator):

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

## Deploy with Nix

Pin shikigami as a flake input:

```nix
{
  inputs.shikigami.url = "github:caiocdcs/shikigami";
}
```

Run it as a systemd service. There is no NixOS module yet, so point a unit at
the flake's binary:

```nix
systemd.services.shikigami = {
  description = "Shikigami heartbeat monitor";
  wantedBy = [ "multi-user.target" ];
  after = [ "network.target" ];
  serviceConfig = {
    ExecStart = "${inputs.shikigami.packages.${pkgs.system}.default}/bin/shikigami";
    DynamicUser = true;
    StateDirectory = "shikigami";
    Restart = "on-failure";
    Environment = [
      "DATABASE_URL=sqlite:/var/lib/shikigami/shikigami.db?mode=rwc"
      "PORT=3000"
      "LOG_LEVEL=info"
    ];
  };
};
```

`StateDirectory` creates `/var/lib/shikigami` for the SQLite file, owned by the
dynamic user.

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

### Ingress (called by monitored jobs)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/ping/{id}` | Heartbeat (creates check-in with outcome=success) |
| POST | `/success/{id}` | Explicit success report |
| POST | `/failure/{id}` | Failure report (triggers notification) |

`{id}` is either the monitor UUID or its slug, so `POST /ping/my-job` works as well as
`POST /ping/<uuid>`. Slugs are restricted to `[A-Za-z0-9_-]` (length 1-50).

## Example: nightly backup monitor

```bash
# 1. Create the monitor (daily at 03:00, with 1-hour grace)
MON_ID=$(curl -s -X POST http://localhost:3000/monitors \
  -H 'Content-Type: application/json' \
  -d '{"name":"nightly-backup","slug":"nightly-backup","schedule_type":"cron","cron_expr":"0 3 * * *","timezone":"America/Sao_Paulo","grace_seconds":3600}' \
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

## Stack

Rust 2024 / Tokio / Axum / sqlx / SQLite / reqwest

## License

[MIT](LICENSE)
