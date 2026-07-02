# Deploy with Docker

Shikigami ships as a multi-arch container image on `ghcr.io`. The image is
self-contained: SQLite is bundled into the binary, so the runtime image needs
only glibc (no `libsqlite3`).

## Image

```
ghcr.io/caiocdcs/shikigami:<tag>
```

Tags:

| Tag | Meaning |
|-----|---------|
| `latest` | Most recent release |
| `0.6.0` | Pinned to a specific release |

Pin to a specific version for reproducibility; use `latest` only for throwaway
or rolling homelab deploys.

## Quickstart (Docker Compose)

```sh
mkdir -p data
curl -O https://raw.githubusercontent.com/caiocdcs/shikigami/main/examples/docker-compose.yml
docker compose up -d
```

This bind-mounts `./data` from the host into the container's
`/var/lib/shikigami`, where the `shikigami.db` file lives. The compose file is
canonical at [`examples/docker-compose.yml`](../../examples/docker-compose.yml);
the docs link here rather than duplicating a snippet that drifts.

Browse to `http://localhost:3000`. CRUD is open by default with a startup
warning; set `API_KEY` to protect management endpoints (see below).

## Baked-in defaults

The image sets these so a bare `docker run` works with no `-e`:

- `DATABASE_URL=sqlite:/var/lib/shikigami/shikigami.db?mode=rwc`
- `LOG_LEVEL=info`

Override any of them, and set the rest, via `environment:` or `env_file:`.

## Data path and volume permissions

The container runs as a non-root user with a fixed UID/GID **1000** and writes
its database to `/var/lib/shikigami` (declared as a `VOLUME`). Bind-mount a host
directory there:

```yaml
volumes:
  - ./data:/var/lib/shikigami
```

The `./data` directory should be owned by host UID 1000 (most homelab hosts'
primary user). If it is not, either `chown` it:

```sh
mkdir -p data && sudo chown 1000:1000 data
```

or override the container user to match your host UID:

```yaml
user: "1000:1000"   # or whatever your host uid is
```

## Configuration

Non-secret overrides go in `environment:`:

```yaml
environment:
  UI_ENABLED: "true"        # serve read-only /status page
  RETENTION_DAYS: "30"      # 0 disables check-in pruning
  LOG_LEVEL: "info"
```

Secrets (`API_KEY`) should NOT be put in `environment:` (they end up in the
container's inspectable config). Use an env file instead:

```yaml
env_file:
  - shikigami.env     # contains: API_KEY=<your-key>  ; chmod 0600
```

See the full environment variable table in the [README](../../README.md).

## Running with plain `docker`

```sh
mkdir -p data
docker run -d \
  --name shikigami \
  -p 3000:3000 \
  -v "$PWD/data:/var/lib/shikigami" \
  --restart unless-stopped \
  ghcr.io/caiocdcs/shikigami:0.6.0
```

## Health probes

The image does not bake in a `HEALTHCHECK` (that would require `curl`/`wget` in
the image, growing it). Use the HTTP endpoints from an external probe or
orchestrator:

- `GET /health` — liveness
- `GET /health/ready` — readiness (verifies the DB)

## Why not a `.env` file mounted into the container?

The binary's dotenvy loader looks for `/.env` in the container, which is not
where you'd typically mount a host `.env`. Prefer `environment:` / `env_file:`
in compose, or `Environment=`/`EnvironmentFile=` with systemd. This keeps the
configuration path explicit and portable.
