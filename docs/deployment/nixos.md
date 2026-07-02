# Deploy on NixOS

Shikigami ships a NixOS module alongside the flake. The package is built from
source with [crane](https://crane.dev), so `nix run` always serves the current
tree rather than a pinned prebuilt tarball.

## Consuming the module

Add the flake as an input and import `nixosModules.default` into your
configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    shikigami = {
      url = "github:caiocdcs/shikigami";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, shikigami, ... }:
    {
      nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          shikigami.nixosModules.default
          ./configuration.nix
        ];
      };
    };
}
```

Then enable the service in `configuration.nix`:

```nix
{ ... }:
{
  services.shikigami = {
    enable = true;
    port = 3000;
    openFirewall = true;
  };
}
```

Rebuild and switch:

```sh
sudo nixos-rebuild switch --flake .#myhost
```

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `services.shikigami.enable` | bool | `false` | Enable the Shikigami service. |
| `services.shikigami.package` | package | flake `packages.default` | Shikigami binary to run. |
| `services.shikigami.port` | port | `3000` | TCP listen port. Passed as `PORT`. |
| `services.shikigami.openFirewall` | bool | `false` | Open the port in `networking.firewall`. |
| `services.shikigami.environment` | attrsOf str | `{}` | Non-secret env overrides (`KEY=VALUE`). |
| `services.shikigami.environmentFile` | nullOr path | `null` | `EnvironmentFile=` for secrets (e.g. `API_KEY`). |

## Non-secret configuration

Put non-secret overrides in `environment`. These become systemd `Environment=`
lines:

```nix
services.shikigami.environment = {
  UI_ENABLED = "true";
  RETENTION_DAYS = "30";
  LOG_LEVEL = "info,sqlx=warn";
};
```

## Secrets (API_KEY)

Never put `API_KEY` in `environment`. Values there are copied into the Nix
store, which is world-readable on the host. Use `environmentFile` instead. It
is passed straight to systemd as `EnvironmentFile=` and never touches the Nix
store.

Create the secret file (e.g. via `sops-nix`, `agenix`, or a plain file with
restricted permissions):

```sh
# /etc/shikigami.env
install -m 0600 /dev/null /etc/shikigami.env
cat > /etc/shikigami.env <<'EOF'
API_KEY=change-me
EOF
chmod 0600 /etc/shikigami.env
```

Point the module at it:

```nix
services.shikigami.environmentFile = /etc/shikigami.env;
```

The service runs as a dynamic user, so make sure the file is readable by root
(systemd reads `EnvironmentFile=` as root before dropping privileges). A file
mode `0600` owned by root satisfies this.

## Data path

The service uses `DynamicUser` with `StateDirectory=shikigami`, so the SQLite
database lives at a fixed path:

```
/var/lib/shikigami/shikigami.db
```

Migrations run automatically on first start. The directory is owned by the
dynamic user and persists across restarts; back it up as needed.

## Logs

```sh
journalctl -u shikigami -f
```

To see startup configuration (env, package version):

```sh
journalctl -u shikigami --no-pager -b
```

## Health probes

The binary exposes:

- `GET /health` - liveness
- `GET /health/ready` - readiness (verifies the DB)

Point an external probe or the NixOS `systemd` watchdog at these. The module
does not configure a health check by default.
