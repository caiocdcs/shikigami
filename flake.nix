{
  description = "Shikigami - self-hosted heartbeat and cron monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
      flake-utils,
      crane,
      ...
    }:
    {
      # System-agnostic NixOS module. Consumed via
      # inputs.shikigami.nixosModules.default from a host flake.
      nixosModules.default = import ./nix/module.nix { inherit self; };
    }
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        toolchain = fenix.packages.${system}.stable.toolchain;
        craneLib = (crane.mkLib pkgs).overrideToolchain (_p: toolchain);

        # Build from source. Keep only the files the build needs so the
        # derivation input changes only when the source actually changes.
        # Required at build time: Cargo.toml, Cargo.lock, src/, .sqlx/,
        # migrations/, templates/. Everything else is excluded.
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            ./.sqlx
            ./migrations
            ./templates
          ];
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          doCheck = false;
          # sqlx compile-time query checking uses the prepared data in .sqlx/
          SQLX_OFFLINE = "true";
          buildInputs = lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        # Build dependencies once and reuse the artifacts. This avoids a
        # cargoHash pin: crane vendors deps from Cargo.lock automatically.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        shikigami = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "shikigami";
            version = "0.6.0";
            meta = {
              mainProgram = "shikigami";
              platforms = lib.platforms.linux;
              description = "Self-hosted heartbeat and cron monitor";
              license = lib.licenses.mit;
            };
          }
        );
      in
      {
        packages.default = shikigami;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            toolchain
            sqlx-cli
            cargo-watch
            cargo-audit
            just
            sqlite
          ];
          SQLX_OFFLINE = "true";
          DATABASE_URL = "sqlite:shikigami.db?mode=rwc";
          shellHook = ''
            echo "shikigami dev shell"
          '';
        };
      }
    );
}
