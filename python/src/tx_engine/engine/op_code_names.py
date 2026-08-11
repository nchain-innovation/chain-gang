""" OP code number to name mapping.

This mapping is derived from the constants in ``op_codes`` so the two can never
drift out of sync. ``op_codes`` is the single source of truth for opcode values.
"""
from typing import Dict

from tx_engine.engine import op_codes as _op_codes

# When several names share the same byte value, the name to report for that byte.
_CANONICAL_NAME: Dict[int, str] = {
    0x00: "OP_0",  # also OP_FALSE
    0x51: "OP_1",  # also OP_TRUE
}


def _build_op_code_names() -> Dict[int, str]:
    names: Dict[int, str] = {}
    # ``vars`` preserves definition order, so the first name wins for any value
    # not listed in ``_CANONICAL_NAME``.
    for name, value in vars(_op_codes).items():
        if not name.startswith("OP_") or not isinstance(value, int):
            continue
        if value in _CANONICAL_NAME:
            names[value] = _CANONICAL_NAME[value]
        elif value not in names:
            names[value] = name
    return names


OP_CODE_NAMES: Dict[int, str] = _build_op_code_names()
