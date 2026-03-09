{ buildEnv
, makeWrapper
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

  nativeBuildInputs = [ makeWrapper ];

  postBuild = ''
    mkdir -p $out/bin

    makeWrapper ${lib.getExe rich-presence-wrapper} $out/bin/zeditor \
      --add-flag zeditor \
      --add-flag --foreground \
      --set _zeditor ${lib.getExe zed-editor}
  '';

  inherit (zed-editor) meta;
}
