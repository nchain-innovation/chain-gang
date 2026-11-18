"""Differential probes for current Chronicle signature code and early exit."""
from harness import (
    K1, K1PUB, RPCError, Script, TxIn, block_check, dsha, ec, fund,
    hashes, log, mine_check, pools, raw, rpc, struct, tx_for, utils, varint,
)

ALL_BIP143 = 0x41
ALL_OTDA = 0x61
ORDER = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
SEP = b"\xab"


def script(data):
    result = Script()
    result.append_data(data)
    return result


def push(data):
    result = Script()
    result.append_pushdata(data)
    return result.raw_serialize()


def sign(tx, parent, code, flag, declared_length=None):
    """Independent ALL preimages for this matrix's single-input spends."""
    outpoint = bytes.fromhex(parent.id())[::-1] + bytes(4)
    sequence = struct.pack("<I", tx.tx_ins[0].sequence)
    version = struct.pack("<I", tx.version)
    lock_and_flag = struct.pack("<II", tx.locktime, flag)
    output = tx.tx_outs[0]
    script_bytes = output.script_pubkey.raw_serialize()
    serialized_output = struct.pack("<q", output.amount) + varint(len(script_bytes)) + script_bytes
    length = len(code) if declared_length is None else declared_length
    sized_code = varint(length) + code
    if flag == ALL_OTDA:
        preimage = (version + b"\x01" + outpoint + sized_code + sequence
                    + b"\x01" + serialized_output + lock_and_flag)
    else:
        preimage = (version + dsha(outpoint) + dsha(sequence) + outpoint + sized_code
                    + struct.pack("<q", parent.tx_outs[0].amount) + sequence
                    + dsha(serialized_output) + lock_and_flag)
    der = K1.sign(dsha(preimage), ec.ECDSA(utils.Prehashed(hashes.SHA256()), deterministic_signing=True))
    r, s = utils.decode_dss_signature(der)
    return utils.encode_dss_signature(r, min(s, ORDER - s)) + bytes([flag])


def run_case(name, parent, tx, expected, expected_reason=None):
    try:
        rpc_result = rpc("sendrawtransaction", raw(tx))
        rpc_error = None
    except RPCError as exc:
        rpc_result = None
        rpc_error = str(exc)
    accepted = tx.id() in pools()["regular"]
    assert accepted == expected, (name, rpc_error)
    if accepted:
        assert rpc_result == tx.id(), (name, rpc_result)
        mine_check(name + "-mined", tx)
    else:
        assert expected_reason in (rpc_error or ""), (name, rpc_error)
        result = block_check(name + "-block", [tx], False)
        assert result["submit_result"] == "blk-bad-inputs", result
    try:
        # The standalone #250 Python API supplies current BSV script rules.
        # Rust receipt replay separately checks the consensus-only API.
        tx.validate([parent])
        library, library_error = True, None
    except ValueError as exc:
        library, library_error = False, str(exc)
    log({"name": name, "txid": tx.id(), "tx": raw(tx), "parents": [raw(parent)],
         "mempool": "regular" if accepted else "rejected", "rpc_error": rpc_error,
         "library_policy": library, "library_error": library_error})
    assert library == expected, (name, accepted, library_error)


def return_tail(flag, use_node_bytes, tail=b"\x4c\x05\xab\x97", consumed=2):
    # OP_RETURN permits opaque bytes after the verified signature. A skipped
    # separator stays in BIP143 and is removed only by the OTDA serializer.
    prefix = b"\x61" + SEP
    skipped_separator = b"\x00\x63" + SEP + b"\x68"
    check = push(K1PUB) + b"\xad\x51\x6a"
    code = skipped_separator + check + tail
    parent = fund(script(prefix + code))
    tx = tx_for([parent])
    if flag == ALL_OTDA:
        without_separator = b"\x00\x63\x68" + check
        declared = len(without_separator) + len(tail)
        serialized_code = without_separator + (tail[:consumed] if use_node_bytes else tail)
    else:
        declared = len(code)
        serialized_code = code if use_node_bytes else b"\x00\x63\x68" + check + tail
        if not use_node_bytes:
            declared = len(serialized_code)
    signature = sign(tx, parent, serialized_code, flag, declared)
    tx.tx_ins = [TxIn(parent.id(), 0, script(push(signature)), 0xFFFFFFFF)]
    run_case(f"return-tail-{tail.hex()}-{flag:02x}-node-{use_node_bytes}",
             parent, tx, use_node_bytes, "CHECKSIGVERIFY")


def nested_return(flag, truncated):
    tail = b"\x4c\x05\xab\x97" if truncated else push(b"\xab\x97")
    code = push(K1PUB) + b"\xad\x51\x51\x63\x6a\x68" + tail
    parent = fund(script(SEP + code))
    tx = tx_for([parent])
    serialized = code[:-2] if flag == ALL_OTDA and truncated else code
    signature = sign(tx, parent, serialized, flag, len(code))
    tx.tx_ins = [TxIn(parent.id(), 0, script(push(signature)), 0xFFFFFFFF)]
    run_case(f"nested-return-{flag:02x}-truncated-{truncated}",
             parent, tx, not truncated, "Opcode missing or not understood")


def mixed_matching(phase, first_flag, poisoned, changed):
    second_flag = ALL_BIP143 if first_flag == ALL_OTDA else ALL_OTDA
    flags = (first_flag, second_flag)
    # The last pair is malformed and rejects only when both earlier pairs
    # matched. Reaching it distinguishes failure at the first or second pair
    # from a generic CHECKMULTISIG failure.
    poison = b"\x30\x01"
    prefix = b"\x61" + SEP
    branch = b"\x00\x63" + SEP + b"\x68"
    pushed_separator = push(SEP)
    cleanup = push(poison) + b"\x75" if poisoned else b""
    count = b"\x53" if poisoned else b"\x52"
    suffix = (pushed_separator + b"\x75" + count
              + push(K1PUB) * (3 if poisoned else 2) + count
              + (b"\xae\x91" if poisoned else b"\xaf\x51"))
    executed = prefix + branch + cleanup + suffix
    # The unlock-phase check signs the whole locking script, including its
    # separator if BIP143 is selected. Keep it as a separate source of bytes.
    lock = executed if phase == "lock" else b"\xab\x69\x51"
    parent = fund(script(lock))
    tx = tx_for([parent])
    signatures = []
    for index, flag in enumerate(flags):
        select_branch = branch if flag == ALL_BIP143 else b"\x00\x63\x68"
        lock_tail = b"" if phase == "lock" else (lock if flag == ALL_BIP143 else lock[1:])
        signed_suffix = suffix
        if index == changed:
            # Change only the bytes signed, not the DER encoding or type.
            # BIP143 must retain the skipped separator. OTDA must retain a
            # pushed 0xab byte even though it removes separator opcodes.
            if flag == ALL_BIP143:
                select_branch = b"\x00\x63\x68"
            else:
                signed_suffix = push(b"\xaa") + suffix[len(pushed_separator):]
        signatures.append(sign(tx, parent, select_branch + (b"\x75" if poisoned else b"")
                               + signed_suffix + lock_tail, flag))
    unlocking = b"\x00" + (push(poison) if poisoned else b"")
    unlocking += push(signatures[1]) + push(signatures[0])
    if phase == "unlock":
        unlocking += executed
    tx.tx_ins = [TxIn(parent.id(), 0, script(unlocking), 0xFFFFFFFF)]
    expected = (changed is not None) if poisoned else (changed is None)
    reason = "Non-canonical DER" if poisoned else "CHECKMULTISIGVERIFY"
    run_case(f"mixed-{phase}-{first_flag:02x}-poison-{poisoned}-changed-{changed}",
             parent, tx, expected, reason)


if __name__ == "__main__":
    import os

    if rpc("getblockcount") == 0:
        rpc("generate", 600)
    probe = os.environ.get("SEPARATOR_PROBE")
    if probe == "node-tail":
        return_tail(ALL_OTDA, True)
    elif probe == "full-tail":
        return_tail(ALL_OTDA, False)
    elif probe == "nested":
        nested_return(ALL_BIP143, True)
    else:
        # Distinguish missing length bytes, a complete length header with too
        # little payload, and an ordinary one-byte push with too little data.
        for flag in (ALL_OTDA, ALL_BIP143):
            for tail, cursor in [(b"\x4c\x05\xab\x97", 2), (b"\x4d\xab", 1),
                                 (b"\x04\xab", 1), (b"\x4e\x05\x00\x00\x00\xab", 5)]:
                for expected in (True, False):
                    return_tail(flag, expected, tail, cursor)
            for truncated in (False, True):
                nested_return(flag, truncated)
        for phase in ("lock", "unlock"):
            for first_flag in (ALL_OTDA, ALL_BIP143):
                for poisoned in (False, True):
                    for changed in (None, 0, 1):
                        mixed_matching(phase, first_flag, poisoned, changed)
        log({"name": "separator-exit-finished", "result": "passed"})
