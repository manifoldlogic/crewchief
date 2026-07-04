import os
import some_external_lib
from pkg.utils import helper


def helper():
    """Local decoy — must NEVER be the import target for b.py's `helper` import.

    Defined after the import so it tends to receive a higher chunk id than
    pkg/utils.py's `helper`; a worktree-global ORDER BY id DESC lookup would pick
    this one. Module-path-scoped resolution must pick pkg/utils.py's instead.
    """
    return "b.local.helper"


def use_b():
    return helper()


def touch_external():
    # References the external imports so they are genuinely used, but neither
    # `os` nor `some_external_lib` is an indexed file, so no edge may reference them.
    return os.getpid() and some_external_lib
