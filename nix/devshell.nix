{ mkShell
, rustToolchain
, lib
, helix
, zed-editor
, playerctl
, ...
}:

mkShell {
  packages = [
    rustToolchain
    helix
  ];

  env = {
    _hx = lib.getExe helix;
    _zeditor = lib.getExe zed-editor;
    _playerctl = lib.getExe playerctl;
  };
}
