{
  stdenv,
  rustPlatform,
  lib,
  scope,

  withHelix ? true,
  withZed ? true,
  withMprisBridge ? stdenv.hostPlatform.isLinux,
  withLsp ? true,

  # Inputs
  nukeRefsElf,
  makeBinaryWrapper,
  git,
  playerctl,
  ...
}:

let
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
        ];
      };

    buildNoDefaultFeatures = true;

    buildFeatures =
      [ ]
      ++ (lib.optional withHelix "helix")
      ++ (lib.optional withZed "zed")
      ++ (lib.optional withMprisBridge "mpris-bridge")
      ++ (lib.optional withLsp "lsp");

    buildInputs =
      [ ]
      ++ (lib.optionals withHelix [ git ])
      ++ (lib.optionals withZed [ git ])
      ++ (lib.optionals withMprisBridge [ playerctl ])
      ++ (lib.optionals withLsp [ ]);

    nativeBuildInputs = [
      nukeRefsElf
      makeBinaryWrapper
    ];

    doCheck = false;

    postInstall = ''
      nukeRefsElf $out/bin/${final.meta.mainProgram}

      wrapProgram $out/bin/${final.meta.mainProgram} \
        --inherit-argv0 \
        --prefix PATH : ${lib.makeBinPath final.buildInputs}
    '';

    cargoLock = {
      lockFile = ../../Cargo.lock;

      outputHashes = {
        "darwin-libproc-0.2.0" = "sha256-jpAyODhGAFuFOjqwGdYcAIHVz/aT+IzyfzJ6Ostj2Yg=";
      };
    };

    meta = {
      description = manifest.package.description or null;
      homepage = manifest.package.homepage or null;
      license = lib.licenses.asl20;
      mainProgram = final.pname;
      platforms = lib.platforms.all;
    };

    passthru =
      let
        callPackage = x: extraArgs: scope.callPackage x ({ inherit rich-presence-wrapper; } // extraArgs);
      in
      { }
      // (lib.optionalAttrs withHelix { helix = callPackage ./helix.nix { }; })
      // (lib.optionalAttrs withZed { zed-editor = callPackage ./zed-editor.nix { }; });
  });
in

rich-presence-wrapper
