from pkg.mod import p_helper


def p_caller():
    # Cross-file call resolved via the post-pass: p_helper is defined in pkg/mod.py.
    return p_helper()


def p_driver():
    # A NON-test caller of p_caller, so the context bundle has a distinct
    # `caller` item (the test caller is claimed by the `test` segment first).
    return p_caller()
