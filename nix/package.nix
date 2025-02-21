{ rustToolchain
, makeRustPlatform
, makeWrapper
, helix
, lib
, ...
}:
with builtins;

let
  manifest = fromTOML (readFile ../Cargo.toml);
  rustPlatform = makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };

  wrappedPrograms = [
    { program = "${helix}/bin/hx"; package = helix; }
  ];
in

rustPlatform.buildRustPackage rec {
  pname = manifest.package.name;
  version = manifest.package.version;
  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  doCheck = false;

  buildInputs = [] ++ (map (x: x.package) wrappedPrograms);
  nativeBuildInputs = [ makeWrapper ];

  postInstall =
    let
      wrapProgram = { program, ... }: let
          programName = baseNameOf program;
        in "makeWrapper $out/bin/${pname} $out/bin/${programName} --inherit-argv0 --set _${programName} ${program}";
    in
      lib.concatLines (map wrapProgram wrappedPrograms);
  
  meta = with lib; {
    description = "Discord rich presence wrapper";
    homepage = "https://github.com/threadexio/rich-presence-wrapper";
    license = licenses.asl20;
    mainProgram = "rich-presence-wrapper";
    platforms = platforms.all;
  };
}
