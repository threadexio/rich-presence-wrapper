import os
from contextlib import closing
from functools import wraps
from subprocess import check_output
from typing import Any, AnyStr, Dict

import tomllib

###############################################################################

_CACHE = {}


def cached():
    def decorator(f):
        @wraps(f)
        def wrapper():
            global _CACHE
            if f.__name__ not in _CACHE:
                _CACHE[f.__name__] = f()

            return _CACHE[f.__name__]

        return wrapper

    return decorator


###############################################################################


@cached()
def repo_root() -> str:
    return os.path.realpath(
        os.path.join(os.path.dirname(os.path.realpath(__file__)), os.pardir)
    )


###############################################################################


def cargo_manifest(path: AnyStr) -> Dict[str, Any]:
    with closing(open(path, "rb")) as f:
        return tomllib.load(f)


@cached()
def workspace_manifest() -> Dict[str, Any]:
    return cargo_manifest(os.path.join(repo_root(), "Cargo.toml"))


@cached()
def package_manifest() -> Dict[str, Any]:
    return workspace_manifest()


###############################################################################


@cached()
def vcs_commit_id() -> str:
    return check_output(
        ["git", "rev-parse", "--short", "HEAD"], encoding="utf-8"
    ).strip()


@cached()
def vcs_is_dirty() -> bool:
    git_status = check_output(
        ["git", "status", "--porcelain"], encoding="utf-8"
    ).strip()
    return len(git_status) > 0
