{
  description = "slykey - minimal Rust text expander CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
  }:
    (flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
        };
        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            libx11
            libxi
            libxtst
            xdotool
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        slykey = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "slykey";
            version = "0.1.0";
          }
        );
      in {
        packages.default = slykey;

        apps.default = flake-utils.lib.mkApp {
          drv = slykey;
        };

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            rust-analyzer
            clippy
            rustfmt
            cargo-watch
            cargo-nextest
            cargo-edit
            cargo-audit
            cargo-deny
            cargo-expand
            libx11
            libxi
            libxtst
            xdotool
            pkg-config
          ];
        };
      }
    ))
    // {
      homeManagerModules.default = import ./nix/home-manager.nix {
        inherit self;
      };
    };
}
