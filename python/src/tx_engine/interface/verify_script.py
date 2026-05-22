"""Helpers for BSV node ``verifyscript`` RPC and Chronicle validation in chain-gang.

**Local validation (chain-gang):** ``Tx.validate()`` and ``Context`` do **not** use
``ScriptFlags``. Chronicle script rules (two-phase eval, malleability relaxation,
32 MB script numbers, high-S verify) are gated on ``tx.version > 1``. Rust also
supports height-aware checks via ``Tx::validate_at_height()`` (Rust) or
``Tx.validate_at_height()`` (Python); ``Tx.validate()`` uses version-only gating.
See ``docs/Chronicle.md``.

**Node RPC:** ``ScriptFlags`` documents flag bits for ``verifyscript`` calls through
``RPCInterface`` to a bitcoin-sv node. The node applies its own Chronicle activation
at consensus block heights; do not assume the same bitmask controls chain-gang eval.
"""
from enum import Enum
from typing import Any, Dict, Final, Optional

# BSV Chronicle activation heights (consensus). See docs/Chronicle.md.
CHRONICLE_ACTIVATION_MAINNET: Final = 943_835
CHRONICLE_ACTIVATION_TESTNET: Final = 1_713_022


def activation_height(network: Optional[str]) -> Optional[int]:
    """Return the Chronicle activation height for a BSV network name, if known."""
    if network is None:
        return None
    normalized = network.lower().replace("_", "")
    if normalized in ("bsvmainnet", "mainnet", "main"):
        return CHRONICLE_ACTIVATION_MAINNET
    if normalized in ("bsvtestnet", "testnet", "test", "bsvstn", "stn"):
        return CHRONICLE_ACTIVATION_TESTNET
    return None


def chronicle_rules_active(
    tx_version: int,
    block_height: Optional[int] = None,
    network: Optional[str] = None,
) -> bool:
    """Whether Chronicle script rules apply (mirrors ``chain_gang::chronicle``).

    When ``block_height`` and ``network`` are omitted, returns ``tx_version > 1``
    (chain-gang ``Tx.validate()`` default). When both are set on a BSV network,
    activation height is enforced.
    """
    if tx_version <= 1:
        return False
    if block_height is None and network is None:
        return True
    if block_height is not None and network is not None:
        threshold = activation_height(network)
        if threshold is None:
            return False
        return block_height >= threshold
    return True


def effective_chronicle_tx_version(
    tx_version: int,
    block_height: Optional[int] = None,
    network: Optional[str] = None,
) -> int:
    """Script version for Chronicle rules after optional activation context."""
    if chronicle_rules_active(tx_version, block_height, network):
        return tx_version
    if tx_version > 1:
        return 1
    return tx_version


class ScriptFlags(int, Enum):
    """Bitcoin-SV node ``verifyscript`` flag bits (RPC reference only).

    These flags are passed to a **node** ``verifyscript`` RPC call. They are **not**
    used by chain-gang's Rust/Python script interpreter. For local validation, set
    ``tx.version`` and use ``Tx.validate()`` / ``Context(tx_version=...)`` instead.

    Legacy node flags such as ``CLEANSTACK``, ``MINIMALIF``, ``NULLFAIL``, and
    ``SIGPUSHONLY`` correspond to malleability rules that chain-gang applies
    automatically from ``tx.version`` when ``chronicle_rules_active()`` is true.
    """

    SCRIPT_VERIFY_NONE = 0

    # Evaluate P2SH subscripts (BIP16).
    SCRIPT_VERIFY_P2SH = 1 << 0

    # Strict signature and pubkey encoding (BIP62-related).
    SCRIPT_VERIFY_STRICTENC = 1 << 1

    # Strict DER signatures (BIP62 rule 1).
    SCRIPT_VERIFY_DERSIG = 1 << 2

    # Low-S signatures (BIP62 rule 5). Relaxed for Chronicle txs (version > 1) on-node.
    SCRIPT_VERIFY_LOW_S = 1 << 3

    # CHECKMULTISIG dummy must be empty (BIP62 rule 7). Relaxed locally when Chronicle active.
    SCRIPT_VERIFY_NULLDUMMY = 1 << 4

    # scriptSig must be push-only (BIP62 rule 2). Relaxed locally when Chronicle active.
    SCRIPT_VERIFY_SIGPUSHONLY = 1 << 5

    # Minimal push and number encodings (BIP62 rules 3–4). Relaxed locally when Chronicle active.
    SCRIPT_VERIFY_MINIMALDATA = 1 << 6

    SCRIPT_VERIFY_DISCOURAGE_UPGRADABLE_NOPS = 1 << 7

    # Exactly one true stack item (BIP62 rule 6). Relaxed locally when Chronicle active.
    SCRIPT_VERIFY_CLEANSTACK = 1 << 8

    SCRIPT_VERIFY_CHECKLOCKTIMEVERIFY = 1 << 9
    SCRIPT_VERIFY_CHECKSEQUENCEVERIFY = 1 << 10

    # OP_IF/NOTIF operand must be empty or 0x01. Relaxed locally when Chronicle active.
    SCRIPT_VERIFY_MINIMALIF = 1 << 13

    # Failed CHECK(MULTI)SIG requires empty signature. Relaxed locally when Chronicle active.
    SCRIPT_VERIFY_NULLFAIL = 1 << 14

    SCRIPT_VERIFY_COMPRESSED_PUBKEYTYPE = 1 << 15
    SCRIPT_ENABLE_SIGHASH_FORKID = 1 << 16
    SCRIPT_GENESIS = 1 << 18
    SCRIPT_UTXO_AFTER_GENESIS = 1 << 19
    SCRIPT_FLAG_LAST = 1 << 20


# Typical post-Genesis BSV node verifyscript flags (not used by chain-gang eval).
DEFAULT_NODE_VERIFYSCRIPT_FLAGS: Final = int(ScriptFlags.SCRIPT_VERIFY_P2SH | ScriptFlags.SCRIPT_VERIFY_DERSIG | ScriptFlags.SCRIPT_VERIFY_LOW_S | ScriptFlags.SCRIPT_VERIFY_NULLDUMMY | ScriptFlags.SCRIPT_VERIFY_SIGPUSHONLY | ScriptFlags.SCRIPT_VERIFY_MINIMALDATA | ScriptFlags.SCRIPT_VERIFY_CLEANSTACK | ScriptFlags.SCRIPT_VERIFY_MINIMALIF | ScriptFlags.SCRIPT_VERIFY_NULLFAIL | ScriptFlags.SCRIPT_ENABLE_SIGHASH_FORKID | ScriptFlags.SCRIPT_GENESIS)


def verifyscript_params(
    tx_hash: str,
    index: int,
    lock_script: str,
    lock_script_amt: int,
    block_height: int = -1,
    script_flags: int = -1,
    report_flags: bool = False,
) -> Dict[str, Any]:
    """Build a ``verifyscript`` RPC request payload for a bitcoin-sv node.

    ``script_flags`` uses :class:`ScriptFlags` node bits. It does not configure
    chain-gang local script evaluation; use ``Tx.validate()`` for that.

    When ``script_flags`` is ``-1``, ``DEFAULT_NODE_VERIFYSCRIPT_FLAGS`` is not
    inserted (same as before: omit flags and let the node default).
    """
    scripts: Dict[str, Any] = {"tx": tx_hash, "n": index}
    if script_flags > -1:
        scripts["flags"] = script_flags
    scripts["reportflags"] = report_flags
    scripts["txo"] = {"lock": lock_script, "value": lock_script_amt, "height": block_height}
    return scripts
