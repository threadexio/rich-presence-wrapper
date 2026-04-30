{
  rustPlatform,
  lib,
  ...
}:

let
  manifest = lib.importTOML ../../Cargo.toml;
in

rustPlatform.buildRustPackage {
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

  cargoLock.lockFile = ../../Cargo.lock;

  meta = {
    description = manifest.package.description or null;
  };
}
