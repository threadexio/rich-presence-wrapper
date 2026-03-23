{ rustPlatform
, callPackage
, makeBinaryWrapper
, lib
, git
, playerctl
, withHelix ? true
, withZed ? true
, withMprisBridge ? true
, ...
}:

let
  manifest = lib.importTOML ../Cargo.toml;

  rich-presence-wrapper = rustPlatform.buildRustPackage (final: {
    pname = manifest.package.name;
    inherit (manifest.package) version;

    src =
      with lib.fileset;
      toSource {
        root = ../.;
        fileset = unions [
          ../src
          ../Cargo.toml
          ../Cargo.lock
        ];
      };

    cargoLock.lockFile = ../Cargo.lock;

    buildNoDefaultFeatures = true;
    buildFeatures = [ ]
      ++ (lib.optional withHelix "helix")
      ++ (lib.optional withZed "zed")
      ++ (lib.optional withMprisBridge "mpris-bridge")
    ;

    buildInputs = [ ]
      ++ (lib.optionals withHelix [ git ])
      ++ (lib.optionals withZed [ git ])
      ++ (lib.optionals withMprisBridge [ playerctl ])
    ;

    nativeBuildInputs = [ makeBinaryWrapper ];

    doCheck = false;

    postInstall = ''
      wrapProgram $out/bin/${final.meta.mainProgram} \
        --inherit-argv0 \
        --prefix PATH : ${lib.makeBinPath final.buildInputs}
    '';

    meta = with lib; {
      description = "Discord rich presence wrapper";
      homepage = "https://github.com/threadexio/rich-presence-wrapper";
      license = licenses.asl20;
      mainProgram = "rich-presence-wrapper";
      platforms = platforms.all;
    };

    passthru =
      let
        args = {
          inherit rich-presence-wrapper;
        };
      in
      { }
      // (lib.optionalAttrs withHelix { helix = callPackage ./helix.nix args; })
      // (lib.optionalAttrs withZed { zed-editor = callPackage ./zed-editor.nix args; })
    ;
  });
in

rich-presence-wrapper
