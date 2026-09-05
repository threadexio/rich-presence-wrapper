{
  self,
  stdenv,
  rustPlatform,
  lib,
  scope,

  rpwConfig ? { },

  # Inputs
  python3,
  nukeRefsElf,
  makeBinaryWrapper,
  git,
  playerctl,
  ...
}:

let
  configType = with lib; {
    external.enable = mkOption {
      type = types.bool;
      default = true;
    };

    helix.enable = mkOption {
      type = types.bool;
      default = true;
    };

    zed.enable = mkOption {
      type = types.bool;
      default = true;
    };

    music-bridge = {
      enable = mkOption {
        type = types.bool;
        default = true;
      };

      source = {
        external.enable = mkOption {
          type = types.bool;
          default = true;
        };

        file.enable = mkOption {
          type = types.bool;
          default = true;
        };

        playerctl.enable = mkOption {
          type = types.bool;
          default = stdenv.hostPlatform.isLinux;
        };
      };

      module = {
        auto-stop.enable = mkOption {
          type = types.bool;
          default = true;
        };

        external.enable = mkOption {
          type = types.bool;
          default = true;
        };

        filter.enable = mkOption {
          type = types.bool;
          default = true;
        };

        fixup-id.enable = mkOption {
          type = types.bool;
          default = true;
        };

        rewrite.enable = mkOption {
          type = types.bool;
          default = true;
        };

        track-position.enable = mkOption {
          type = types.bool;
          default = true;
        };
      };
    };

    lsp.enable = mkOption {
      type = types.bool;
      default = true;
    };
  };

  inherit
    (lib.evalModules {
      modules = [
        {
          options = configType;
        }
        {
          config = rpwConfig;
        }
      ];
    })
    config
    ;

  manifest = lib.importTOML ../../Cargo.toml;

  rich-presence-wrapper = rustPlatform.buildRustPackage (final: {
    pname = manifest.package.name;
    inherit (manifest.package) version;

    src =
      with lib.fileset;
      toSource {
        root = ../../.;
        fileset = unions [
          (maybeMissing ../../.cargo)
          (maybeMissing ../../build.rs)
          ../../src
          ../../Cargo.toml
          ../../Cargo.lock
          ../../scripts
        ];
      };

    cargoLock = {
      lockFile = ../../Cargo.lock;

      outputHashes = {
        "darwin-libproc-0.2.0" = "sha256-jpAyODhGAFuFOjqwGdYcAIHVz/aT+IzyfzJ6Ostj2Yg=";
      };
    };

    buildNoDefaultFeatures = true;

    buildFeatures =
      [ ]
      ++ (lib.optional config.external.enable "external")
      ++ (lib.optional config.helix.enable "helix")
      ++ (lib.optional config.zed.enable "zed")
      ++ (lib.optionals config.music-bridge.enable (
        [ "music-bridge" ]
        ++ [
          (lib.optional config.music-bridge.source.external.enable "music-bridge.source.external")
          (lib.optional config.music-bridge.source.file.enable "music-bridge.source.file")
          (lib.optional config.music-bridge.source.playerctl.enable "music-bridge.source.playerctl")
        ]
        ++ [
          (lib.optional config.music-bridge.module.auto-stop.enable "music-bridge.module.auto-stop")
          (lib.optional config.music-bridge.module.external.enable "music-bridge.module.external")
          (lib.optional config.music-bridge.module.filter.enable "music-bridge.module.filter")
          (lib.optional config.music-bridge.module.fixup-id.enable "music-bridge.module.fixup-id")
          (lib.optional config.music-bridge.module.rewrite.enable "music-bridge.module.rewrite")
          (lib.optional config.music-bridge.module.track-position.enable "music-bridge.module.track-position")
        ]
      ))
      ++ (lib.optional config.lsp.enable "lsp");

    buildInputs =
      [ ]
      ++ (lib.optionals (config.helix.enable or config.zed.enable) [ git ])
      ++ (lib.optionals config.music-bridge.enable (
        [ ] ++ (lib.optionals config.music-bridge.source.playerctl.enable [ playerctl ])
      ));

    nativeBuildInputs = [
      python3
      nukeRefsElf
      makeBinaryWrapper
    ];

    env = {
      COMMIT = lib.elemAt (lib.splitString "-" (self.dirtyShortRev or self.shortRev)) 0;
      DIRTY = if lib.hasAttr "dirtyRev" self then "1" else "0";
    };

    doCheck = false;

    preBuild = ''
      find scripts/ -type f -executable |
        while IFS= read -r f; do
          patchShebangs "$f"
        done
    '';

    postInstall = ''
      nuke-refs-elf $out/bin/${final.meta.mainProgram}

      wrapProgram $out/bin/${final.meta.mainProgram} \
        --inherit-argv0 \
        --prefix PATH : "${lib.makeBinPath final.buildInputs}"
    '';

    meta = {
      description = manifest.package.description or null;
      homepage = manifest.package.homepage or null;
      license = lib.licenses.asl20;
      mainProgram = final.pname;
      platforms = lib.flatten (
        with lib.platforms;
        [
          linux
          darwin
        ]
      );
    };

    passthru =
      let
        callPackage = x: extraArgs: scope.callPackage x ({ inherit rich-presence-wrapper; } // extraArgs);
      in
      {
        inherit config;
      }
      // (lib.optionalAttrs config.helix.enable { helix = callPackage ./helix.nix { }; })
      // (lib.optionalAttrs config.zed.enable { zed-editor = callPackage ./zed-editor.nix { }; });
  });
in

rich-presence-wrapper
