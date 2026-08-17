{
  makeSetupHook,
  writeText,

  # Inputs
  patchelf,
  nukeReferences,
  ...
}:

makeSetupHook
  {
    name = "nuke-refs-elf";

    propagatedBuildInputs = [
      patchelf
      nukeReferences
    ];
  }
  (
    writeText "nuke-refs-elf.sh" ''
      nukeRefsElf() {
        isDynamicElf() {
          readelf -l "''${1:?missing elf}" | grep -q "INTERP"
        }

        local elf="''${1:?missing elf}"

        if isDynamicElf "$elf"; then
          local interpreter="$(patchelf --print-interpreter "$elf")"
          local rpath="$(patchelf --print-rpath "$elf")"

          nuke-refs "$elf"

          patchelf "$elf" \
            --set-interpreter "$interpreter" \
            --set-rpath "$rpath"
        else
          nuke-refs "$elf"
        fi
      }
    ''
  )
