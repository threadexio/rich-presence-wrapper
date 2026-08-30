#!/usr/bin/env python3
import sys
import os

import lib

###############################################################################


def _main() -> int:
    cargo_version = os.environ.get("CARGO_PKG_VERSION") or lib.package_manifest()["package"]["version"]
    commit = os.environ.get("COMMIT") or lib.vcs_commit_id()

    if "DIRTY" in os.environ:
        dirty = os.environ["DIRTY"] != "0"
    else:
        dirty = lib.vcs_is_dirty()

    print(f"{cargo_version}-{commit}{'+' if dirty else ''}")
    return 0


if __name__ == "__main__":
    sys.exit(_main())
