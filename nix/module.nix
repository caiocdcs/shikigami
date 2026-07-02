# Shikigami - NixOS module
#
# Consumed by the flake as:
#   nixosModules.default = import ./nix/module.nix { inherit self; };
#
# The module receives `self` so the default package can resolve to the flake's
# crane-built binary for the host system (NixOS is Linux, so
# self.packages.${pkgs.system}.default is the crane package).
{
  self,
}:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.shikigami;

  # Non-secret environment overrides mapped to KEY=VALUE strings for systemd.
  environmentLines = lib.mapAttrsToList (k: v: "${k}=${v}") cfg.environment;
in
{
  options.services.shikigami = {
    enable = lib.mkEnableOption "shikigami heartbeat and cron monitor";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.system}.default;
      defaultText = lib.literalMD "shikigami flake `packages.default`";
      description = ''
        Shikigami binary package to run. Defaults to the crane-built package
        from this flake. Override to pin a specific version or a local build.
      '';
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = ''
        TCP port the HTTP listener binds to. Passed to the service as PORT.
      '';
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Open the configured port in the firewall (networking.firewall).
      '';
    };

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      example = lib.literalExpression ''
        {
          UI_ENABLED = "true";
          RETENTION_DAYS = "30";
          LOG_LEVEL = "info";
        }
      '';
      description = ''
        Extra non-secret environment variables applied to the service as
        KEY=VALUE. Do NOT put secrets (e.g. API_KEY) here: attrs values are
        copied into the Nix store and are world-readable. Use
        services.shikigami.environmentFile for secrets.
      '';
    };

    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      example = "/run/secrets/shikigami.env";
      description = ''
        Path to an environment file (systemd EnvironmentFile=) holding secret
        variables such as API_KEY. The file must contain lines of the form
        KEY=VALUE and should be chmod 0600. Setting this is the only supported
        way to pass API_KEY to the service without leaking it to the Nix store.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.shikigami = {
      description = "Shikigami heartbeat and cron monitor";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/shikigami";
        DynamicUser = true;
        StateDirectory = "shikigami";
        Restart = "on-failure";
        Environment = [
          "DATABASE_URL=sqlite:/var/lib/shikigami/shikigami.db?mode=rwc"
          "PORT=${toString cfg.port}"
          "LOG_LEVEL=info"
        ]
        ++ environmentLines;
        EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
