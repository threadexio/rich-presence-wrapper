{
  description = "Discord Rich Presence wrapper";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

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
    }:
    let
      systems = [
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-linux"
        "x86_64-darwin"
      ];

      inherit (nixpkgs) lib;

      mkPkgs =
        system:
        import nixpkgs {
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

      perSystem = f: lib.genAttrs systems (system: f (mkPkgs system));
    in
    {
      formatter = perSystem (pkgs: pkgs.nixpkgs-fmt);

      devShells = perSystem (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            helix
          ];

          env._hx = lib.getExe pkgs.helix;
        };
      });

      packages = perSystem (pkgs: {
        default = pkgs.rich-presence-wrapper;

        helix = pkgs.callPackage ./nix/helix.nix { };
        zed-editor = pkgs.callPackage ./nix/zed-editor.nix { };
      });

      apps = perSystem (
        pkgs:
        let
          mkApp = drv: {
            type = "app";
            program = lib.getExe drv;
          };
        in
        lib.mapAttrs (_: mkApp) self.packages.${pkgs.system}
      );

      overlays = {
        default = final: _: {
          rich-presence-wrapper = self.packages.${final.system}.default;
        };

        helix = final: prev: {
          helix = final.callPackage ./nix/helix.nix { inherit (prev) helix; };
        };

        zed-editor = final: prev: {
          zed-editor = final.callPackage ./nix/zed-editor.nix { inherit (prev) zed-editor; };
        };
      };
    };
}
