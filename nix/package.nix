{ rustToolchain
, makeRustPlatform
, lib
, ...
}:

let
  manifest = lib.importTOML ../Cargo.toml;

  rustPlatform = makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };
in

rustPlatform.buildRustPackage {
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
}
