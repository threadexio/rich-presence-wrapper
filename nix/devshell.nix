{ mkShell
, rustToolchain
, lib
, helix
, ...
}:

mkShell {
  packages = [
    rustToolchain
    helix
  ];

  env._hx = lib.getExe helix;
}
