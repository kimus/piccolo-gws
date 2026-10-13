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
        # `gws` handles Workspace API calls, but `gws auth setup` shells out to
        # `gcloud` for Google Cloud/OAuth project setup, so package both in the
        # runtime wrapper PATH. If nixpkgs ever lacks `gws`, keep the package
        # evaluable with the SDK fallback.
        runtimeDeps = [ (pkgs.gws or pkgs.google-cloud-sdk) pkgs.google-cloud-sdk ];
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
