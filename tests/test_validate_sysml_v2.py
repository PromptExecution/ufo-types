"""Cross-runtime parity fixtures for ``validate_sysml_v2`` (P3,
``elasticdotventures/_b00t_#1177``): each case here is the same fixture used
by the native Rust test suite in ``src/sysml.rs``, verbatim — proving the
Python binding agrees with the Rust-owned logic on identical input, not
just that it runs.
"""

from ufo_types import validate_sysml_v2


def test_valid_sysml_v2_is_satisfied():
    # Mirrors sysml.rs::tests::valid_sysml_v2_is_satisfied
    text = (
        "package Foo {\n"
        "    part def Bar {\n"
        "        attribute x : ScalarValues::Boolean;\n"
        "    }\n"
        "}\n"
    )
    is_valid, reason = validate_sysml_v2(text)
    assert is_valid is True
    assert reason is None


def test_sysml_v1_block_def_keyword_is_rejected():
    # Mirrors sysml.rs::tests::sysml_v1_block_def_keyword_is_rejected —
    # SysML v1 called this construct `Block`; SysML v2 renamed it to
    # `part def`. `block def` is not a SysML v2 keyword at all.
    text = "package Foo {\n    block def Bar {\n    }\n}\n"
    is_valid, reason = validate_sysml_v2(text)
    assert is_valid is False
    assert reason is not None


def test_comment_swallowing_closing_brace_is_rejected():
    # Mirrors sysml.rs::tests::comment_swallowing_closing_brace_is_rejected
    # — a `//` line comment on the same line as a closing `}` comments the
    # brace out, leaving the block unclosed.
    text = "package Foo {\n    part def Bar { // note\n}\n"
    is_valid, reason = validate_sysml_v2(text)
    assert is_valid is False
    assert reason is not None
