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
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        lib = pkgs.lib;

        rustVersion = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };

        helix-rich-presence = rustPlatform.buildRustPackage rec {
          pname = "helix-rich-presence";
          version = "0.1.0";
          src = ./.;

          buildInputs = with pkgs; [ helix ];
          nativeBuildInputs = with pkgs; [ makeWrapper ];

          cargoLock.lockFile = ./Cargo.lock;

          postInstall = ''
            wrapProgram $out/bin/${pname} --prefix PATH : ${lib.makeBinPath buildInputs}
          '';
        };
      in
      {
        packages.default = helix-rich-presence;

        apps.default = flake-utils.lib.mkApp {
          drv = helix-rich-presence;
          name = "hx";
        };
      }
    );
}
