{
  description = "Discord Rich Presence wrapper";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

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
    { self
    , nixpkgs
    , rust-overlay
    , ...
    }:
    let
      systems = [
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-linux"
        "x86_64-darwin"
      ];

      wrappedApps = [
        "helix"
        "zed-editor"
      ];

      inherit (nixpkgs) lib;

      mkPkgs =
        system:
        import nixpkgs {
          inherit system;
          overlays = [
            (
              pkgs: _:
                let
                  scope = lib.makeScope pkgs.newScope (scope: {
                    inherit self;

                    rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
                    rustToolchain = scope.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

                    rustPlatform = pkgs.makeRustPlatform {
                      rustc = scope.rustToolchain;
                      cargo = scope.rustToolchain;
                    };

                    devshell = scope.callPackage ./nix/devshell.nix { };
                    rich-presence-wrapper = scope.callPackage ./nix/package.nix { };
                  });
                in
                {
                  inherit (scope)
                    rich-presence-wrapper
                    devshell
                    ;
                }
            )
          ];
        };

      perSystem' = f: lib.genAttrs systems f;
      perSystem = f: perSystem' (system: f (mkPkgs system));

      mkApp = drv: {
        type = "app";
        program = lib.getExe drv;
      };
    in
    {
      formatter = perSystem (pkgs: pkgs.nixpkgs-fmt);

      devShells = perSystem (pkgs: {
        default = pkgs.devshell;
      });

      packages = perSystem (pkgs: {
        default = pkgs.rich-presence-wrapper;
        inherit (pkgs) rich-presence-wrapper;
      }
      // (lib.genAttrs wrappedApps (name: pkgs.rich-presence-wrapper.passthru.${name}))
      );

      apps = perSystem' (system: lib.mapAttrs (_: mkApp) self.packages.${system});

      overlays = {
        default = pkgs: _: {
          rich-presence-wrapper = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
      }
      // (lib.genAttrs wrappedApps (name:
        final: prev: {
          "${name}" = final.callPackage ./nix/${name}.nix {
            "${name}" = lib.getAttr name prev;
          };
        }
      ))
      ;

      homeModules.default = import ./nix/home-module.nix { inherit self; };
    };
}
