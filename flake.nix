{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      fenix,
      flake-utils,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        toolchain = fenix.packages.${system}.stable.toolchain;
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            toolchain
            sqlx-cli
            cargo-watch
            just

            # sqlite
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
