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
      });

      apps = perSystem (pkgs: {
        default = {
          type = "app";
          program = lib.getExe pkgs.rich-presence-wrapper;
        };
      });

      overlays =
        let
          mkOverlay =
            { package
            , pathsToLink ? [ ]
            , wrapperArgs ? [ ]
            , ...
            }:
            final: prev:
            let
              drv = prev.${package};
              programName = drv.meta.mainProgram or drv.pname;

              rich-presence-wrapper =
                lib.throwIfNot (lib.hasAttr "rich-presence-wrapper" final)
                  "The '${package}' overlay of `rich-presence-wrapper` requires that the package `rich-presence-wrapper` is available in `pkgs`. Did you forget to include the 'default' overlay?"
                  final.rich-presence-wrapper;
            in
            {
              "${package}" = final.buildEnv {
                name = "${drv.pname}-rich-presence-wrapper";

                paths = [
                  rich-presence-wrapper
                  drv
                ];

                inherit pathsToLink;

                nativeBuildInputs = [ final.makeWrapper ];

                postBuild = ''
                  mkdir -p $out/bin

                  makeWrapper ${lib.getExe rich-presence-wrapper} $out/bin/${programName} \
                    --set _${programName} ${lib.getExe drv} \
                    --inherit-argv0 \
                    ${lib.concatStringsSep " " wrapperArgs}
                '';
              };
            };
        in
        {
          default = final: _: {
            rich-presence-wrapper = self.packages.${final.system}.default;
          };

          helix = mkOverlay {
            package = "helix";

            pathsToLink = [
              "/share"
              "/lib"
            ];
          };

          zed-editor = mkOverlay {
            package = "zed-editor";

            pathsToLink = [
              "/share"
              "/libexec"
            ];

            wrapperArgs = [
              "--add-flags"
              "--foreground"
            ];
          };
        };
    };
}
