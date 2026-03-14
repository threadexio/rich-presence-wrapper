{ buildEnv
, makeBinaryWrapper
, lib
, rich-presence-wrapper
, helix
, ...
}:

buildEnv {
  name = "helix-rich-presence-wrapper";

  paths = [
    rich-presence-wrapper
    helix
  ];

  pathsToLink = [
    "/share"
    "/lib"
  ];

  nativeBuildInputs = [ makeBinaryWrapper ];

  postBuild = ''
    mkdir -p $out/bin

    makeBinaryWrapper ${lib.getExe rich-presence-wrapper} $out/bin/hx \
      --inherit-argv0 \
      --set _hx ${lib.getExe helix}
  '';

  inherit (helix) meta;
}
