{ buildEnv
, makeBinaryWrapper
, lib
, rich-presence-wrapper
, zed-editor
, ...
}:

buildEnv {
  name = "zed-editor-rich-presence-wrapper";

  paths = [
    rich-presence-wrapper
    zed-editor
  ];

  pathsToLink = [
    "/share"
    "/libexec"
  ];

  nativeBuildInputs = [ makeBinaryWrapper ];

  postBuild = ''
    mkdir -p $out/bin

    makeBinaryWrapper ${lib.getExe rich-presence-wrapper} $out/bin/zeditor \
      --inherit-argv0 \
      --set _zeditor ${lib.getExe zed-editor}
  '';

  inherit (zed-editor) meta;
}
