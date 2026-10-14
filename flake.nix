{
  description = "Google Workspace integration package for Piccolo";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        # Runtime installs the lightweight `gws` backend only. OAuth client
        # credentials are created once in Google Cloud Console and imported via
        # `piccolo-gws setup`, so the multi-gigabyte Google Cloud SDK is not a
        # dependency of this package or its release bundles.
        runtimeDeps = [ pkgs.gws ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "piccolo-gws";
          version = "0.2.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postInstall = ''
            wrapProgram $out/bin/piccolo-gws \
              --prefix PATH : ${pkgs.lib.makeBinPath runtimeDeps}
          '';
        };

        apps.default = flake-utils.lib.mkApp { drv = self.packages.${system}.default; };

        devShells.default = pkgs.mkShell {
          packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.nix ];
        };
      });
}
