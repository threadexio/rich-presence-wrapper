{
  stdenv,
  writeShellScriptBin,
  lib,

  # Inputs
  patchelf,
  nukeReferences,
  gawk,
  darwin,
  ...
}:

let
  signingUtils =
    if (stdenv.targetPlatform.isDarwin && stdenv.targetPlatform.isAarch64) then
      darwin.signingUtils
    else
      null;
in

writeShellScriptBin "nuke-refs-elf" ''
  set -e

  ${lib.optionalString (!isNull signingUtils) ''
    fixupHooks=()
    source "${signingUtils}"
  ''}

  exe="''${1:?missing executable}"
  shift

  ${lib.optionalString stdenv.targetPlatform.isLinux ''
    interp="$(${lib.getExe patchelf} --print-interpreter "$exe")"
    IFS=$':' read -ra rpaths < <(${lib.getExe patchelf} --print-rpath "$exe")

    nuke_refs_args=("-e" "$interp")

    for rpath in "''${rpaths[@]}"; do
      nuke_refs_args+=("-e" "$rpath")
    done

    ${lib.getExe nukeReferences} "''${nuke_refs_args[@]}" -- "$exe"
  ''}

  ${lib.optionalString stdenv.targetPlatform.isDarwin ''
    mapfile -d $'\0' -t dylibs < <(otool -L "$exe" | tail -n +2 | ${lib.getExe gawk} '{printf $1 "\0"}')

    nuke_refs_args=()

    for dylib in "''${dylibs[@]}"; do
      if [[ "$dylib" != "${builtins.storeDir}"* ]]; then
        continue
      fi

      nuke_refs_args+=("-e" "$dylib")
    done

    ${lib.getExe nukeReferences} "''${nuke_refs_args[@]}" -- "$exe"

    ${lib.optionalString (!isNull signingUtils) ''
      signIfRequired "$exe"
    ''}
  ''}
''
