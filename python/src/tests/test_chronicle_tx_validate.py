""" Chronicle integration tests through Tx.validate and Context
"""
import unittest

from tx_engine import Context, Script, SIGHASH, Tx, TxIn, TxOut, Wallet
from tx_engine.interface.verify_script import CHRONICLE_ACTIVATION_MAINNET

SECP256K1_N = int(
    "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141", 16
)


def display_tx_hash(tx: Tx) -> str:
    """Return the display-order txid hex string used by TxIn.prev_tx."""
    return bytes(reversed(bytes(tx.hash()))).hex()


def _read_push(data: bytes, offset: int = 0) -> tuple[bytes, int]:
    op = data[offset]
    if 1 <= op <= 75:
        length = op
        start = offset + 1
        return data[start:start + length], start + length
    if op == 0x4C:
        length = data[offset + 1]
        start = offset + 2
        return data[start:start + length], start + length
    raise ValueError(f"unsupported push opcode 0x{op:02x}")


def _push_bytes(data: bytes) -> bytes:
    length = len(data)
    if length <= 75:
        return bytes([length]) + data
    if length <= 255:
        return bytes([0x4C, length]) + data
    raise ValueError("push too large for test helper")


def _der_split_rs(der: bytes) -> tuple[bytes, bytes]:
    assert der[0] == 0x30
    idx = 2
    assert der[idx] == 0x02
    r_len = der[idx + 1]
    r = der[idx + 2:idx + 2 + r_len]
    idx = idx + 2 + r_len
    assert der[idx] == 0x02
    s_len = der[idx + 1]
    s = der[idx + 2:idx + 2 + s_len]
    return r, s


def _der_join(r: bytes, s: bytes) -> bytes:
    body = b"\x02" + bytes([len(r)]) + r + b"\x02" + bytes([len(s)]) + s
    return b"\x30" + bytes([len(body)]) + body


def flip_der_to_high_s(der: bytes) -> bytes:
    r, s_bytes = _der_split_rs(der)
    s = int.from_bytes(s_bytes, "big")
    if s > SECP256K1_N // 2:
        raise ValueError("expected low-S DER input")
    high_s = SECP256K1_N - s
    s_enc = high_s.to_bytes((high_s.bit_length() + 7) // 8, "big")
    if s_enc[0] & 0x80:
        s_enc = b"\x00" + s_enc
    return _der_join(r, s_enc)


def replace_first_push_with_high_s(script: Script) -> Script:
    sig_with_type, rest_offset = _read_push(bytes(script.cmds))
    der, sighash_type = sig_with_type[:-1], sig_with_type[-1]
    high_sig = flip_der_to_high_s(der) + bytes([sighash_type])
    pubkey, _ = _read_push(bytes(script.cmds), rest_offset)
    return Script(list(_push_bytes(high_sig)) + list(_push_bytes(pubkey)))


class ChronicleTxValidateTest(unittest.TestCase):
    """ End-to-end Chronicle behavior via native Tx.validate
    """

    def _fund_and_spend(
        self,
        lock_script: Script,
        unlock_script: Script,
        spend_version: int,
    ) -> tuple[Tx, Tx]:
        fund = Tx(
            version=1,
            tx_ins=[],
            tx_outs=[TxOut(amount=1_000, script_pubkey=lock_script)],
        )
        spend = Tx(
            version=spend_version,
            tx_ins=[
                TxIn(
                    display_tx_hash(fund),
                    0,
                    unlock_script,
                )
            ],
            tx_outs=[TxOut(amount=900, script_pubkey=Script([]))],
        )
        return fund, spend

    def test_two_phase_functional_unlock_validates(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_5 OP_EQUAL"),
            Script.parse_string("OP_2 OP_3 OP_ADD"),
            2,
        )
        self.assertIsNone(spend.validate([fund]))

    def test_version_one_rejects_functional_unlock(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_5 OP_EQUAL"),
            Script.parse_string("OP_2 OP_3 OP_ADD"),
            1,
        )
        with self.assertRaises(ValueError):
            spend.validate([fund])

    def test_relaxed_clean_stack_validates_for_version_two(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_1 OP_1"),
            Script.parse_string("OP_1"),
            2,
        )
        self.assertIsNone(spend.validate([fund]))

    def test_relaxed_clean_stack_rejects_version_one(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_1 OP_1"),
            Script.parse_string("OP_1"),
            1,
        )
        with self.assertRaises(ValueError):
            spend.validate([fund])

    def test_context_two_phase_matches_tx_validate_path(self):
        unlock = Script.parse_string("OP_2 OP_3 OP_ADD")
        lock = Script.parse_string("OP_5 OP_EQUAL")
        self.assertTrue(
            Context(script=unlock, lock_script=lock, tx_version=2).evaluate()
        )

    def test_validate_at_height_rejects_pre_activation(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_5 OP_EQUAL"),
            Script.parse_string("OP_2 OP_3 OP_ADD"),
            2,
        )
        self.assertIsNone(
            spend.validate_at_height(
                [fund],
                CHRONICLE_ACTIVATION_MAINNET,
                "BSV_Mainnet",
            )
        )
        with self.assertRaises(ValueError):
            spend.validate_at_height(
                [fund],
                CHRONICLE_ACTIVATION_MAINNET - 1,
                "BSV_Mainnet",
            )

    def test_high_s_p2pkh_validates_for_chronicle_tx(self):
        private_key = int.from_bytes(bytes([2] * 32), "big")
        wallet = Wallet.from_int("BSV_Mainnet", private_key)

        fund = Tx(
            version=1,
            tx_ins=[],
            tx_outs=[TxOut(amount=10, script_pubkey=wallet.get_locking_script())],
        )
        spend = Tx(
            version=2,
            tx_ins=[
                TxIn(
                    display_tx_hash(fund),
                    0,
                    Script([]),
                )
            ],
            tx_outs=[TxOut(amount=5, script_pubkey=Script([]))],
        )
        signed = wallet.sign_tx_sighash(
            0, fund, spend, int(SIGHASH.ALL_FORKID_CHRONICLE)
        )
        high_s_unlock = replace_first_push_with_high_s(
            signed.tx_ins[0].script_sig
        )
        signed.tx_ins[0].script_sig = high_s_unlock
        self.assertIsNone(signed.validate([fund]))


if __name__ == "__main__":
    unittest.main()
