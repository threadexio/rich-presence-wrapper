{
  stdenv,
  mkShell,
  rustToolchain,
  lib,

  # Inputs
  helix,
  zed-editor,
  playerctl,
  ...
}:

mkShell {
  packages = [
    rustToolchain
  ];

  env = {
    _hx = lib.getExe helix;
    _zeditor = lib.getExe zed-editor;
  }
  // (lib.optionalAttrs stdenv.hostPlatform.isLinux {
    _playerctl = lib.getExe playerctl;
  });
}
