from enum import Enum


class SIGHASH(int, Enum):
    """ SigHash flags for details see https://wiki.bitcoinsv.io/index.php/SIGHASH_flags
    """
    ALL: int = 0x01
    NONE: int = 0x02
    SINGLE: int = 0x03
    ANYONECANPAY: int = 0x80

    FORKID: int = 0x40

    ALL_FORKID = ALL | FORKID
    NONE_FORKID = NONE | FORKID
    SINGLE_FORKID = SINGLE | FORKID
    ALL_ANYONECANPAY_FORKID = ALL_FORKID | ANYONECANPAY
    NONE_ANYONECANPAY_FORKID = NONE_FORKID | ANYONECANPAY
    SINGLE_ANYONECANPAY_FORKID = SINGLE_FORKID | ANYONECANPAY
