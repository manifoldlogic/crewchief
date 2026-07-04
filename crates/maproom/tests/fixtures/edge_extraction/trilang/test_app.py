from app import p_caller


def test_p_caller():
    # `test_` prefix -> classified as a test; calling p_caller yields a test_of edge.
    assert p_caller() == 3
    return p_caller()
