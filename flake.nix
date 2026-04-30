{
  description = "A very basic flake.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";

    flake-compat = {
      url = "github:NixOS/flake-compat";
      flake = false;
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }@inputs:
    let
      inherit (nixpkgs) lib;

      systems = lib.systems.flakeExposed;
      perSystem = f: lib.genAttrs systems f;
      perSystem' =
        f:
        perSystem (
          system:
          f (
            import nixpkgs {
              inherit system;

              overlays = [
                (pkgs: _: {
                  scope = lib.makeScope pkgs.newScope (scope: {
                    inherit self inputs;

                    rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
                    rustToolchain = scope.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

                    rustPlatform = pkgs.makeRustPlatform {
                      rustc = scope.rustToolchain;
                      cargo = scope.rustToolchain;
                    };

                    devShells = {
                      default = scope.callPackage ./nix/devshells { };
                    };

                    packages' = {
                      default = scope.callPackage ./nix/packages { };
                    };

                    apps = lib.mapAttrs (_: package: {
                      type = "app";
                      program = "${lib.getExe package}";
                    }) scope.packages';
                  });
                })
              ];
            }
          )
        );
    in
    {
      formatter = perSystem' (pkgs: pkgs.nixfmt-tree);

      devShells = perSystem' (pkgs: pkgs.scope.devShells);
      packages = perSystem' (pkgs: pkgs.scope.packages');
      apps = perSystem' (pkgs: pkgs.scope.apps);
    };
}
