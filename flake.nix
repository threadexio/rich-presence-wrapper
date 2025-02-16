{
  description = "Discord Rich Presence for the helix editor";

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
            helix-rich-presence = final.callPackage ./nix/package.nix { };
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

        packages.default = pkgs.helix-rich-presence;

        apps.default = flake-utils.lib.mkApp {
          drv = pkgs.helix-rich-presence;
          name = "hx";
        };
      }
    );
}
