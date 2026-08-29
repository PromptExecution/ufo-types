"""Cross-runtime SysML v2 syntax validation, backed by ufo-types' Rust-owned
``validate_sysml_v2`` (P3, ``elasticdotventures/_b00t_#1177``).
"""

from __future__ import annotations

import importlib

_core = None


def _get_core():
    global _core
    if _core is not None:
        return _core
    try:
        _core = importlib.import_module("ufo_types._core")
    except ImportError as exc:
        raise RuntimeError(
            "Native ufo_types._core is unavailable. "
            "Build it with: maturin develop --features python"
        ) from exc
    return _core


def validate_sysml_v2(text: str) -> tuple[bool, str | None]:
    """Validate ``text`` as SysML v2 syntax via the real ``sysml-v2-parser``
    grammar. Returns ``(is_valid, reason)`` — ``reason`` is ``None`` when
    valid, and the joined parser diagnostic message(s) otherwise.
    """
    return _get_core().validate_sysml_v2(text)


def version() -> str:
    return _get_core().version()
