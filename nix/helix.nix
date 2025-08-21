{
  buildEnv,
  makeWrapper,
  lib,

  rich-presence-wrapper,
  helix ? null,
  ...
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

  nativeBuildInputs = [ makeWrapper ];

  postBuild = ''
    mkdir -p $out/bin

    makeWrapper ${lib.getExe rich-presence-wrapper} $out/bin/hx \
      --set _hx ${lib.getExe helix} \
      --inherit-argv0
  '';

  inherit (helix) meta;
}
