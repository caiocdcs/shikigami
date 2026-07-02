# Deploy with systemd

For a bare-metal or VM Linux host, deploy the prebuilt binary with a systemd
unit. The binary is self-contained (SQLite is bundled), so the host needs only
glibc and the binary itself.

## 1. Install the binary

One-line install (Linux x86_64/aarch64), which verifies the sha256sum:

```sh
curl -sSfL https://github.com/caiocdcs/shikigami/releases/latest/download/install.sh | sh
```

If you prefer to inspect the script first (recommended), download it:

```sh
curl -sSfL https://github.com/caiocdcs/shikigami/releases/latest/download/install.sh -o install.sh
less install.sh
sh install.sh
```

This installs the binary to `/usr/local/bin/shikigami`.

## 2. Install the unit

The canonical unit lives at
[`examples/shikigami.service`](../../examples/shikigami.service). Copy it into
place:

```sh
sudo install -m 0644 examples/shikigami.service /etc/systemd/system/shikigami.service
sudo systemctl daemon-reload
```

The unit uses `DynamicUser=yes` + `StateDirectory=shikigami`, so systemd:

- creates an ephemeral, dedicated user (no static uid to manage),
- creates `/var/lib/shikigami` and chowns it to that user,
- tears down the user on stop.

This matches the NixOS and Docker deployments, which all use
`/var/lib/shikigami` as the data path.

## 3. Configure

Non-secret options are set via `Environment=` in the unit. For secrets (notably
`API_KEY`), use an `EnvironmentFile=` so the secret never lives in the unit
file or the journal:

```sh
sudo install -m 0600 /dev/null /etc/shikigami/env
sudo tee -a /etc/shikigami/env >/dev/null <<'EOF'
API_KEY=change-me
EOF
sudo chmod 0600 /etc/shikigami/env
```

Then uncomment the `EnvironmentFile=` line in the installed unit:

```ini
EnvironmentFile=/etc/shikigami/env
```

Reload and start:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now shikigami
```

## 4. Verify

```sh
systemctl status shikigami
curl -s http://localhost:3000/health
journalctl -u shikigami -f
```

## Data path and backups

The database lives at `/var/lib/shikigami/shikigami.db`. Migrations run
automatically on first start. Back up that directory (or at least the `.db`
file) on your normal schedule.

## Health probes

- `GET /health` - liveness
- `GET /health/ready` - readiness (verifies the DB)

Point an external monitor or your existing uptime tool at these.
