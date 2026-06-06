{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      flake-utils,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        version = "0.1.1";
        arch =
          {
            x86_64-linux = "x86_64-linux";
            aarch64-linux = "aarch64-linux";
          }
          .${system} or null;

        sha256 =
          {
            x86_64-linux = "9d76e7d95059e47811961727b9df8791afc99a9de837467331f9240e9033f684";
            aarch64-linux = "5e9a4c4efa91f94cc0542c3e8def017b6a8b4a3029bfdf889e5b09544fc760a0";
          }
          .${arch} or null;

      in
      pkgs.lib.optionalAttrs (arch != null) {
        packages.default = pkgs.stdenv.mkDerivation {
          pname = "shikigami";
          inherit version;
          src = pkgs.fetchurl {
            url = "https://github.com/caiocdcs/shikigami/releases/download/v${version}/shikigami-${arch}.tar.gz";
            inherit sha256;
          };
          sourceRoot = ".";
          nativeBuildInputs = [ pkgs.patchelf ];
          installPhase = ''
            install -Dm755 shikigami -t $out/bin
            patchelf --set-interpreter "$(cat $NIX_CC/nix-support/dynamic-linker)" \
              --set-rpath "${pkgs.stdenv.cc.cc.lib}/lib" $out/bin/shikigami
          '';
        };
      }
    );
}
