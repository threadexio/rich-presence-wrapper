{
  symlinkJoin,
  lib,

  # Inputs
  makeBinaryWrapper,
  rich-presence-wrapper,
  helix,
  ...
}:

symlinkJoin {
  name = "helix-rich-presence-wrapper";

  paths = [
    helix
  ];

  nativeBuildInputs = [ makeBinaryWrapper ];

  postBuild = ''
    makeBinaryWrapper ${lib.getExe rich-presence-wrapper} $out/bin/hx \
      --inherit-argv0 \
      --set _hx ${lib.getExe helix}
  '';
}
