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
in

rustPlatform.buildRustPackage rec {
  pname = manifest.package.name;
  version = manifest.package.version;
  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  buildInputs = [ helix ];
  nativeBuildInputs = [ makeWrapper ];

  postInstall = ''
    wrapProgram $out/bin/${pname} --set HELIX ${helix}/bin/hx
    ln -rsf $out/bin/${pname} $out/bin/hx
  '';

  meta = with lib; {
    description = "Discord rich presence for the helix editor";
    homepage = "https://github.com/threadexio/helix-rich-presence";
    license = licenses.asl20;
    mainProgram = "hx";
    platforms = platforms.all;
  };
}
