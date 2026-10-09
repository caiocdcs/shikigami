# Deploy with systemd

For a non-NixOS Linux host, build the binary and run it with a systemd unit.
The binary is self-contained (SQLite is bundled), so the host needs only glibc
and the binary itself.

## 1. Build and install the binary

With Nix:

```sh
nix build github:caiocdcs/shikigami
sudo install -m 0755 result/bin/shikigami /usr/local/bin/shikigami
```

Or from source with a Rust toolchain:

```sh
cargo build --release
sudo install -m 0755 target/release/shikigami /usr/local/bin/shikigami
```

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

This matches the NixOS deployment, which also uses
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

The database lives at `/var/lib/shikigami/shikigami.db`. It runs in WAL mode,
which keeps recent writes in `shikigami.db-wal` alongside the main file -- so
copying only `shikigami.db` while the service is running misses recent writes.
Back up with the SQLite online backup (`sqlite3 ... ".backup ..."`) or by
stopping the service first and copying all three files. See the
[README](../../README.md#database-file--backups) for details. Do not store the
database on a network filesystem (NFS/SMB). Migrations run automatically on
first start.

## Health probes

- `GET /health` - liveness
- `GET /health/ready` - readiness (verifies the DB)

Point an external monitor or your existing uptime tool at these.
