{
  symlinkJoin,
  lib,

  # Inputs
  makeBinaryWrapper,
  rich-presence-wrapper,
  zed-editor,
  ...
}:

symlinkJoin {
  name = "zed-editor-rich-presence-wrapper";

  paths = [
    zed-editor
  ];

  nativeBuildInputs = [ makeBinaryWrapper ];

  postBuild = ''
    makeBinaryWrapper ${lib.getExe rich-presence-wrapper} $out/bin/zeditor \
      --inherit-argv0 \
      --set _zeditor ${lib.getExe zed-editor}
  '';
}
