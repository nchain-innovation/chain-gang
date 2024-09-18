""" OP codes as first class Python constants
"""

from typing import Final
# Used to convert OP name to code

OP_0: Final = 0
OP_FALSE: Final = 0
OP_PUSHDATA1: Final = 0x4C
OP_PUSHDATA2: Final = 0x4D
OP_PUSHDATA4: Final = 0x4E
OP_1NEGATE: Final = 0x4F
OP_RESERVED: Final = 0x50
OP_TRUE: Final = 0x51
OP_1: Final = 0x51
OP_2: Final = 0x52
OP_3: Final = 0x53
OP_4: Final = 0x54
OP_5: Final = 0x55
OP_6: Final = 0x56
OP_7: Final = 0x57
OP_8: Final = 0x58
OP_9: Final = 0x59
OP_10: Final = 0x5A
OP_11: Final = 0x5B
OP_12: Final = 0x5C
OP_13: Final = 0x5D
OP_14: Final = 0x5E
OP_15: Final = 0x5F
OP_16: Final = 0x60

# Control
OP_NOP: Final = 0x61
OP_VER: Final = 0x62
OP_IF: Final = 0x63
OP_NOTIF: Final = 0x64
OP_VERIF: Final = 0x65
OP_VERNOTIF: Final = 0x66
OP_ELSE: Final = 0x67
OP_ENDIF: Final = 0x68
OP_VERIFY: Final = 0x69
OP_RETURN: Final = 0x6A

# stack ops
OP_TOALTSTACK: Final = 0x6B
OP_FROMALTSTACK: Final = 0x6C
OP_2DROP: Final = 0x6D
OP_2DUP: Final = 0x6E
OP_3DUP: Final = 0x6F
OP_2OVER: Final = 0x70
OP_2ROT: Final = 0x71
OP_2SWAP: Final = 0x72
OP_IFDUP: Final = 0x73
OP_DEPTH: Final = 0x74
OP_DROP: Final = 0x75
OP_DUP: Final = 0x76
OP_NIP: Final = 0x77
OP_OVER: Final = 0x78
OP_PICK: Final = 0x79
OP_ROLL: Final = 0x7A
OP_ROT: Final = 0x7B
OP_SWAP: Final = 0x7C
OP_TUCK: Final = 0x7D

# Splice ops - BSV
OP_CAT: Final = 0x7E
OP_SPLIT: Final = 0x7F
OP_NUM2BIN: Final = 0x80
OP_BIN2NUM: Final = 0x81
OP_SIZE: Final = 0x82
# 0x83{131} .. 0x86{134} - transaction invalid for non BSV

# Bit logic - BSV
OP_INVERT: Final = 0x83
OP_AND: Final = 0x84
OP_OR: Final = 0x85
OP_XOR: Final = 0x86
OP_EQUAL: Final = 0x87
OP_EQUALVERIFY: Final = 0x88
OP_RESERVED1: Final = 0x89
OP_RESERVED2: Final = 0x8A

# Artithmetic
OP_1ADD: Final = 0x8B
OP_1SUB: Final = 0x8C
OP_2MUL: Final = 0x8D  # Disabled
OP_2DIV: Final = 0x8E  # Disabled
OP_NEGATE: Final = 0x8F
OP_ABS: Final = 0x90
OP_NOT: Final = 0x91
OP_0NOTEQUAL: Final = 0x92
OP_ADD: Final = 0x93
OP_SUB: Final = 0x94
OP_MUL: Final = 0x95
OP_DIV: Final = 0x96
OP_MOD: Final = 0x97
OP_LSHIFT: Final = 0x98
OP_RSHIFT: Final = 0x99

OP_BOOLAND: Final = 0x9A
OP_BOOLOR: Final = 0x9B
OP_NUMEQUAL: Final = 0x9C
OP_NUMEQUALVERIFY: Final = 0x9D
OP_NUMNOTEQUAL: Final = 0x9E
OP_LESSTHAN: Final = 0x9F
OP_GREATERTHAN: Final = 0xA0
OP_LESSTHANOREQUAL: Final = 0xA1
OP_GREATERTHANOREQUAL: Final = 0xA2
OP_MIN: Final = 0xA3
OP_MAX: Final = 0xA4
OP_WITHIN: Final = 0xA5
OP_RIPEMD160: Final = 0xA6
OP_SHA1: Final = 0xA7
OP_SHA256: Final = 0xA8
OP_HASH160: Final = 0xA9
OP_HASH256: Final = 0xAA
OP_CODESEPARATOR: Final = 0xAB
OP_CHECKSIG: Final = 0xAC
OP_CHECKSIGVERIFY: Final = 0xAD
OP_CHECKMULTISIG: Final = 0xAE
OP_CHECKMULTISIGVERIFY: Final = 0xAF

# Expansion
OP_NOP1: Final = 0xB0
OP_CHECKLOCKTIMEVERIFY: Final = 0xB1
OP_CHECKSEQUENCEVERIFY: Final = 0xB2
