{ rustPlatform
, callPackage
, lib
, ...
}:

let
  manifest = lib.importTOML ../Cargo.toml;

  rich-presence-wrapper = rustPlatform.buildRustPackage {
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

    doCheck = false;

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
  };
in

rich-presence-wrapper
