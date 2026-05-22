""" Chronicle Python Context parity tests
"""
import unittest

from tx_engine import Context, Script
from tx_engine.engine.op_codes import (
    OP_BIN2NUM,
    OP_ENDIF,
    OP_IF,
    OP_PUSHDATA1,
    OP_PUSHDATA4,
    OP_1,
)
from tx_engine.engine.util import max_script_num_length, MAX_SCRIPT_NUM_LENGTH_CHRONICLE

# Valid DER sig + compressed pubkey from test_sighash (verify fails with wrong z).
_FAILED_CHECKSIG_SCRIPT = Script.parse_string(
    "0x3045022100aacb290ed3aeb43fc91a179d6a3ffef4c5efcca612c901c719e198c2ee685e2702200505bd74db673c6d723a141bd8ac469327ea4bb7987110042f23f8c3d7f91e3d41 "
    "0x03dcf21dbdbaa744333af236c3382c85d6308e6d05599df5d3cb19e0f19a205d43 "
    "OP_CHECKSIG OP_DROP OP_1"
)
_WRONG_Z = bytes(32)


def _push_data4_zeros(size: int) -> Script:
    return Script(
        [
            OP_PUSHDATA4,
            *size.to_bytes(4, "little"),
            b"\x00" * size,
            OP_BIN2NUM,
            OP_1,
        ]
    )


class ChronicleContextTest(unittest.TestCase):
    """ Chronicle rules exposed through Python Context and util helpers
    """

    def test_op_ver_with_tx_version(self):
        script = Script.parse_string("OP_VER OP_2 OP_NUMEQUAL")
        self.assertTrue(Context(script=script, tx_version=2).evaluate())
        self.assertFalse(Context(script=script, tx_version=1).evaluate())

    def test_op_verif_executes_when_version_is_high_enough(self):
        script = Script.parse_string("OP_2 OP_VERIF OP_1 OP_ELSE OP_0 OP_ENDIF")
        self.assertTrue(Context(script=script, tx_version=2).evaluate())
        self.assertFalse(Context(script=script, tx_version=1).evaluate())

    def test_op_vernotif_executes_when_version_is_too_low(self):
        script = Script.parse_string("OP_2 OP_VERNOTIF OP_1 OP_ELSE OP_0 OP_ENDIF")
        self.assertTrue(Context(script=script, tx_version=1).evaluate())
        self.assertFalse(Context(script=script, tx_version=2).evaluate())

    def test_two_phase_carries_stack(self):
        unlock = Script.parse_string("OP_2 OP_3 OP_ADD")
        lock = Script.parse_string("OP_5 OP_EQUAL")
        ctx = Context(script=unlock, lock_script=lock, tx_version=2)
        self.assertTrue(ctx.evaluate())
        self.assertEqual(ctx.get_stack().size(), 1)

    def test_two_phase_clears_alt_stack(self):
        unlock = Script.parse_string("OP_1 OP_TOALTSTACK OP_2 OP_3 OP_ADD")
        lock = Script.parse_string("OP_5 OP_EQUAL")
        ctx = Context(script=unlock, lock_script=lock, tx_version=2)
        self.assertTrue(ctx.evaluate())
        self.assertEqual(ctx.get_altstack().size(), 0)

    def test_relaxed_clean_stack(self):
        script = Script.parse_string("OP_1 OP_2 OP_1")
        self.assertFalse(Context(script=script, tx_version=1).evaluate())
        self.assertTrue(Context(script=script, tx_version=2).evaluate())

    def test_chronicle_minimalif_allows_non_minimal_true_operand(self):
        script = Script(
            [OP_PUSHDATA1, 4, bytes([0, 0, 0, 127]), OP_IF, OP_1, OP_ENDIF, OP_1]
        )
        self.assertTrue(Context(script=script, tx_version=2).evaluate())
        self.assertFalse(Context(script=script, tx_version=1).evaluate())

    def test_chronicle_nullfail_allows_failed_checksig_with_nonempty_sig(self):
        ctx = Context(
            script=_FAILED_CHECKSIG_SCRIPT,
            z=_WRONG_Z,
            tx_version=2,
        )
        self.assertTrue(ctx.evaluate())

    def test_strict_nullfail_rejects_failed_checksig_with_nonempty_sig(self):
        ctx = Context(
            script=_FAILED_CHECKSIG_SCRIPT,
            z=_WRONG_Z,
            tx_version=1,
        )
        self.assertFalse(ctx.evaluate())

    def test_chronicle_script_num_limit_accepts_genesis_max_bin2num(self):
        script = _push_data4_zeros(750_000)
        self.assertTrue(Context(script=script, tx_version=2).evaluate())

    def test_genesis_script_num_limit_rejects_oversized_bin2num(self):
        script = _push_data4_zeros(750_001)
        self.assertFalse(Context(script=script, tx_version=1).evaluate())

    def test_max_script_num_length(self):
        self.assertEqual(max_script_num_length(1), 750_000)
        self.assertEqual(max_script_num_length(2), MAX_SCRIPT_NUM_LENGTH_CHRONICLE)
        self.assertEqual(max_script_num_length(None), 750_000)
        self.assertEqual(max_script_num_length(2, pregenesis=True), 4)


if __name__ == "__main__":
    unittest.main()
