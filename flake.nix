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
        # Prefer a packaged `gws` backend when nixpkgs provides one; fall back to
        # Google Cloud SDK so images still include Google auth tooling.
        runtimeDeps = [ (pkgs.gws or pkgs.google-cloud-sdk) ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "piccolo-gws";
          version = "0.1.0";
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
          packages = [ pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy pkgs.nix pkgs.google-cloud-sdk ];
        };
      });
}
