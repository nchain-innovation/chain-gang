"""Check which transaction fields each signature commits to through Tx.validate."""
import hashlib
import itertools
import json
import os
from pathlib import Path
import struct
import unittest

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec, utils
from tx_engine import Tx


ALL, NONE, SINGLE = 1, 2, 3
CHRONICLE, FORKID, ANYONECANPAY = 0x20, 0x40, 0x80
TRUE, DROP, SEPARATOR = b'\x51', b'\x75', b'\xab'
ZERO_HASH = bytes(32)
MUTATIONS = (
    'none', 'paired-output', 'other-output', 'current-sequence', 'other-sequence',
    'current-prevout', 'other-prevout', 'amount', 'locktime', 'version',
    'signature', 'append-output', 'other-unlock', 'paired-script', 'other-script',
)


def hash256(data):
    return hashlib.sha256(hashlib.sha256(data).digest()).digest()


def u32(value):
    return struct.pack('<I', value)


def counted(data):
    # All vector counts and script lengths in these fixtures use one CompactSize byte.
    assert len(data) < 253
    return bytes([len(data)]) + data


def output_bytes(output):
    amount, script = output
    return struct.pack('<q', amount) + counted(script)


def tx_bytes(version, inputs, outputs, locktime):
    encoded_inputs = b''.join(point + counted(script) + u32(seq) for point, script, seq in inputs)
    encoded_outputs = b''.join(map(output_bytes, outputs))
    return u32(version) + bytes([len(inputs)]) + encoded_inputs + bytes([len(outputs)]) + encoded_outputs + u32(locktime)


def signing_digest(flag, index, inputs, outputs, amount, bip143_code, original_code):
    base, acp = flag & 31, bool(flag & ANYONECANPAY)
    point, _, sequence = inputs[index]
    if flag & CHRONICLE:
        if base == SINGLE and index >= len(outputs):
            return b'\x01' + bytes(31)
        signed_inputs = []
        for i, (prevout, _, seq) in enumerate(inputs):
            if i == index:
                signed_inputs.append((prevout, original_code, seq))
            elif not acp:
                signed_inputs.append((prevout, b'', seq if base == ALL else 0))
        signed_outputs = outputs
        if base == NONE:
            signed_outputs = []
        elif base == SINGLE:
            signed_outputs = [(-1, b'')] * index + [outputs[index]]
        return hash256(tx_bytes(2, signed_inputs, signed_outputs, 0) + u32(flag))

    prevouts = ZERO_HASH if acp else hash256(b''.join(p for p, _, _ in inputs))
    sequences = ZERO_HASH
    if not acp and base == ALL:
        sequences = hash256(b''.join(u32(s) for _, _, s in inputs))
    output_hash = ZERO_HASH
    if base == ALL:
        output_hash = hash256(b''.join(map(output_bytes, outputs)))
    elif base == SINGLE and index < len(outputs):
        output_hash = hash256(output_bytes(outputs[index]))
    preimage = u32(2) + prevouts + sequences + point + counted(bip143_code)
    preimage += struct.pack('<qI', amount, sequence) + output_hash + u32(0) + u32(flag)
    return hash256(preimage)


def preserves_signature(flag, missing, change):
    base = flag & 31
    if change == 'signature':
        return False
    if change == 'append-output':
        return base == NONE or (base == SINGLE and not missing)
    if missing and flag & CHRONICLE:
        return True
    if change in ('paired-output', 'paired-script'):
        return base == NONE
    if change in ('other-output', 'other-script'):
        return base != ALL
    if change == 'other-sequence':
        return bool(flag & ANYONECANPAY) or base != ALL
    if change == 'other-prevout':
        return bool(flag & ANYONECANPAY)
    if change == 'amount':
        return bool(flag & CHRONICLE)
    return change in ('none', 'other-unlock')


class OtdaCommitmentsTest(unittest.TestCase):
    def test_commitment_matrix(self):
        # Fixed test key. Sign the independently serialized digest before mutating the spend.
        key = ec.derive_private_key(int.from_bytes(bytes([9]) * 32, 'big'), ec.SECP256K1())
        pubkey = key.public_key().public_bytes(
            serialization.Encoding.X962, serialization.PublicFormat.CompressedPoint)
        rows = []
        for base, era, acp, index, style in itertools.product(
            (ALL, NONE, SINGLE), (0, CHRONICLE), (0, ANYONECANPAY),
            (0, 1), ('lock', 'unlock', 'mixed-multisig'),
        ):
            flag = base | era | acp | FORKID
            sizes = (3, 1) if base == SINGLE and index == 1 else (3,)
            for size in sizes:
                rows.extend(self.check_spend(key, pubkey, flag, index, style, size))

        self.assertEqual(len(rows), 1236)
        if path := os.environ.get('CHRONICLE_DIFF_OUTPUT'):
            Path(path).write_text(json.dumps(rows))

    def check_spend(self, key, pubkey, flag, index, style, size):
        multi = style == 'mixed-multisig'
        flags = [flag, flag ^ CHRONICLE] if multi else [flag]
        verify = (b'\x52' + counted(pubkey) * 2 + b'\x52\xaf' if multi
                  else counted(pubkey) + b'\xad')

        # Both algorithms retain the pushed 0xab. Only OTDA removes the skipped opcode.
        def code(skipped):
            return verify + b'\x00\x63' + skipped + b'\x68' + counted(SEPARATOR) + DROP + TRUE

        lock = DROP + TRUE if style == 'unlock' else SEPARATOR + code(SEPARATOR)
        suffix = lock if style == 'unlock' else b''
        parents = [tx_bytes(1, [], [(100, lock if i == index else TRUE)], i) for i in range(2)]
        inputs = [(hash256(raw) + u32(0), b'', 100 + i) for i, raw in enumerate(parents)]
        outputs = [(10 + i, TRUE) for i in range(size)]

        def unlocking(amount, wrong_signature=False):
            script = b'\x00' if multi else b''
            for i, f in enumerate(flags):
                digest = signing_digest(f, index, inputs, outputs, amount,
                                        code(SEPARATOR) + suffix, code(b'') + suffix)
                if wrong_signature and i == 0:
                    digest = hash256(digest)
                sig = key.sign(digest, ec.ECDSA(utils.Prehashed(hashes.SHA256()),
                                                deterministic_signing=True))
                script += counted(sig + bytes([f]))
            return script + (SEPARATOR + code(SEPARATOR) if style == 'unlock' else b'')

        unlock = unlocking(100)
        for change in MUTATIONS:
            missing = index >= size
            if missing and change in ('paired-output', 'paired-script'):
                continue
            txins, txouts, funding = inputs.copy(), outputs.copy(), parents.copy()
            version, locktime = 2, 0
            selected = index if not change.startswith('other-') else 1 - index
            point, _, seq = txins[selected]
            actual_unlock = unlock
            if change.endswith('-prevout'):
                funding[selected] = tx_bytes(1, [], [(100, lock if selected == index else TRUE)], 9)
                txins[selected] = (hash256(funding[selected]) + u32(0), b'', seq)
            elif change.endswith('-sequence'):
                txins[selected] = (point, b'', seq + 1)
            elif change in ('paired-output', 'other-output', 'paired-script', 'other-script'):
                value, script = txouts[selected]
                txouts[selected] = ((value, b'\x52') if change.endswith('-script')
                                    else (value + 1, script))
            elif change == 'append-output':
                txouts.append((1, TRUE))
            elif change == 'other-unlock':
                txins[selected] = (point, b'\x00' + DROP, seq)
            elif change == 'amount':
                # Keep the real outpoint fixed and sign with the wrong spent amount.
                actual_unlock = unlocking(101)
            elif change == 'signature':
                actual_unlock = unlocking(100, wrong_signature=True)
            elif change == 'locktime':
                locktime += 1
            elif change == 'version':
                version += 1
            point, _, seq = txins[index]
            txins[index] = (point, actual_unlock, seq)
            raw = tx_bytes(version, txins, txouts, locktime)
            tx = Tx.parse(raw)
            utxos = [Tx.parse(p) for p in funding]
            expected = all(preserves_signature(f, missing, change) for f in flags)
            name = f'{flag:02x}-{index}-{style}-missing={str(missing).lower()}-{change}'
            try:
                tx.validate(utxos)
                error = None
            except ValueError as exc:
                error = str(exc)
            accepted = error is None
            with self.subTest(case=name):
                self.assertEqual(accepted, expected, error)
                if not expected:
                    assert error is not None
                    reason = 'OP_CHECKMULTISIGVERIFY failed' if multi else 'OP_CHECKSIGVERIFY failed'
                    self.assertIn(reason, error)
            yield {'name': name, 'expected': expected, 'library': accepted,
                   'request': {'tx': raw.hex(), 'n': index, 'flags': 3997710,
                               'reportflags': True, 'txo': {'lock': lock.hex(), 'value': 100}}}
