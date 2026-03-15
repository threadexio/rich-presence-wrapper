{ rustPlatform
, callPackage
, makeBinaryWrapper
, lib
, git
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

    buildInputs = [ git ];
    nativeBuildInputs = [ makeBinaryWrapper ];

    doCheck = false;

    postInstall = ''
      wrapProgram $out/bin/${final.meta.mainProgram} \
        --inherit-argv0 \
        --prefix PATH : ${lib.makeBinPath [ git ]}
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
      {
        helix = callPackage ./helix.nix args;
        zed-editor = callPackage ./zed-editor.nix args;
      };
  });
in

rich-presence-wrapper
