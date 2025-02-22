{
  description = "Discord Rich Presence wrapper";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    let
      mkPkgs = system: import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)

          (final: _: {
            rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          })

          (final: _: {
            rich-presence-wrapper = final.callPackage ./nix/package.nix { };
          })
        ];
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = mkPkgs system;
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages.default = pkgs.rich-presence-wrapper;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            helix
          ];

          env._hx = "${pkgs.helix}/bin/hx";
        };

        apps.default = flake-utils.lib.mkApp {
          drv = pkgs.rich-presence-wrapper;
        };
      }
    );
}
