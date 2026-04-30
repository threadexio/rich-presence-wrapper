{
  mkShell,
  rustToolchain,
  ...
}:

mkShell {
  packages = [
    rustToolchain
  ];
}
