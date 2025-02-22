{ rustToolchain
, makeRustPlatform
, makeWrapper
, helix
, lib
, programs ? [ "helix" ]
, ...
}:
with builtins;

let
  manifest = fromTOML (readFile ../Cargo.toml);
  rustPlatform = makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };

  availablePrograms = {
    "helix" = { program = "${helix}/bin/hx"; package = helix; };
  };

  mapEnabledPrograms = f: map (name: f name availablePrograms.${name}) programs;

  pname = manifest.package.name;
  version = manifest.package.version;
in
rustPlatform.buildRustPackage {
  inherit pname version;
  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  inherit programs;
  availablePrograms = attrNames availablePrograms;

  doCheck = false;
  buildNoDefaultFeatures = true;
  buildFeatures = mapEnabledPrograms (name: _: name);
  buildInputs = mapEnabledPrograms (_: { package, ... }: package);
  nativeBuildInputs = [ makeWrapper ];

  postInstall =
    let
      installCmd = { program, ... }:
        "makeWrapper $out/bin/${pname} $out/bin/${baseNameOf program} --inherit-argv0 --set _${baseNameOf program} ${program}";

      installCmds = mapEnabledPrograms (_: installCmd);
    in
    lib.concatLines installCmds;

  meta = with lib; {
    description = "Discord rich presence wrapper";
    homepage = "https://github.com/threadexio/rich-presence-wrapper";
    license = licenses.asl20;
    mainProgram = "rich-presence-wrapper";
    platforms = platforms.all;
  };
}
