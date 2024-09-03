import unittest
import sys

sys.path.append("..")

from tx_engine import Tx, TxOut, TxIn,  Script, sig_hash_preimage, sig_hash, SIGHASH, Context
from tx_engine.engine.util import (
    GROUP_ORDER_INT,
    Gx, 
    Gx_bytes
)

from tx_engine.engine.utility_scripts import ensure_is_positive, reverse_endianness, nums_to_script

# the pushtx function below is part of the script_libraries directory. I've included it here for the unittest.
# the rest of the functions are part chain_gang

def pushtx(sighash, load: bool, clean_stack: bool, sign_code: bool) -> Script:
 # Load variables to altstack ----------------------------------------------------------------------------------------------------------------------------------------------
    load_variables = nums_to_script([Gx,GROUP_ORDER_INT])
    load_variables += Script.parse_string('OP_DUP')
    load_variables.append_pushdata(bytes.fromhex('00' * 32))
    load_variables.append_pushdata(Gx_bytes)
    load_variables += Script.parse_string('OP_DUP 0x0220 OP_SWAP OP_CAT OP_2 OP_CAT OP_SWAP OP_2 OP_SWAP OP_CAT')
    load_variables += Script.parse_string(' '.join(['OP_TOALTSTACK'] * 6))

    # End of load variables to altstack ---------------------------------------------------------------------------------------------------------------------------------------

    out = Script()
    
    # Construct transaction ID
    # After this, the stack is: h
    out += Script.parse_string('OP_HASH256') + reverse_endianness(32) + ensure_is_positive()                                                                                        # Hash message and ensure it's interpreted as positive number                                                                               
    
    # Compute the s part of the signature
    # Altstack required: [GROUP_ORDER_INT, Gx]
    # After this, the stack is: s
    out += Script.parse_string('OP_FROMALTSTACK OP_ADD OP_FROMALTSTACK OP_MOD')                                                                                                     # compute h + G_x % GROUP_ORDER_INT
    
    # Altstack required: [GROUP_ORDER_INT]
    # After this, the stack is: s (canonical form)
    out += Script.parse_string('OP_DUP OP_FROMALTSTACK OP_TUCK OP_2 OP_DIV OP_GREATERTHAN OP_IF OP_SWAP OP_SUB OP_ELSE OP_DROP OP_ENDIF')                                          # bring h + aG_x to canonical form (< GROUP_ORDER_INT / 2)
    
    # After this, the stack is: s (big endian)
    out += Script.parse_string('OP_SIZE OP_SWAP OP_FROMALTSTACK OP_CAT 32 OP_SPLIT OP_DROP') + reverse_endianness(32) + Script.parse_string('0x20 OP_ROT OP_SUB OP_SPLIT OP_NIP')                               
    
    # Altstack required: [0x0220||Gx||02]
    # After this, the stack is: Der(Gx,s) || SIGHASH_FLAG
    out += Script.parse_string('OP_SIZE OP_DUP 36 OP_ADD 48 OP_SWAP OP_CAT OP_FROMALTSTACK OP_CAT OP_SWAP OP_CAT OP_SWAP OP_CAT')                          
    out += Script.parse_string('0x' +  sighash.to_bytes(1,'big').hex() + ' OP_CAT')       

    if load:
        out = load_variables + out

    # Pull private key from the altstack
    out += Script.parse_string('OP_FROMALTSTACK')
    

    if clean_stack and sign_code:
        out += Script.parse_string('OP_CHECKSIGVERIFY')
    elif clean_stack and not sign_code:
        out += Script.parse_string('OP_CODESEPARATOR OP_CHECKSIGVERIFY')
    elif not clean_stack and sign_code:
        out += Script.parse_string('OP_CHECKSIG')
    else:
        out += Script.parse_string('OP_CODESEPARATOR OP_CHECKSIG')

    return out

class SigHashTest(unittest.TestCase):
    def test_sig_hash(self):
        txid = "413c9c771fa794d2b1b1e51a84347f859021b7a1c401701ffaced59c9e71eabd"
        prev_value = 100
        vout = 0
        dummy_tx_in = TxIn(prev_tx=txid,prev_index=vout)
        tx = Tx(version=1, tx_ins=[dummy_tx_in], tx_outs=[], locktime=0)

        pushtx_lock = pushtx(0x41,True,False,True)

        sig_hash_val = sig_hash(tx=tx, index=0, script_pubkey=pushtx_lock, prev_amount=prev_value, sighash_value=SIGHASH.ALL_FORKID)


        preimage_sighash = sig_hash_preimage(tx=tx, index=0, script_pubkey=pushtx_lock, prev_amount=prev_value, sighash_value=SIGHASH.ALL_FORKID)

        script_sig = Script()
        script_sig.append_pushdata(preimage_sighash)
        dummy_tx_in.script_sig = script_sig
        full_script = dummy_tx_in.script_sig + pushtx_lock

        context = Context(script=dummy_tx_in.script_sig + pushtx_lock, z=sig_hash_val)
        #assert(context.evaluate())
        self.assertTrue(context.evaluate())

    def test_sig_hash_hardcoded_values(self):
        txid = "413c9c771fa794d2b1b1e51a84347f859021b7a1c401701ffaced59c9e71eabd"
        prev_value = 100
        vout = 0
        pushtx_lock = pushtx(0x41,True,False,True)

        dummy_tx_in = TxIn(prev_tx=txid,prev_index=vout)

        # Preimage sighash for tx below computed with old tx_engine for SIGHASH_ALL, assuming that dummy_tx_in is locked with pushtx_lock
        preimage_sighash = b'\x01\x00\x00\x00\xddH\xb0\x86_|}\x1fT|H\n\x93p\x19f\xa9\xef\xf1Qt&\x04\xcaE\xcf\x0f\xe5h\x87j\xe5;\xb10)\xce{\x1fU\x9e\xf5\xe7G\xfc\xacC\x9f\x14U\xa2\xec|_\t\xb7"\x90y^pfPD\xbd\xeaq\x9e\x9c\xd5\xce\xfa\x1fp\x01\xc4\xa1\xb7!\x90\x85\x7f4\x84\x1a\xe5\xb1\xb1\xd2\x94\xa7\x1fw\x9c<A\x00\x00\x00\x00\xfd\xa2\x01 \x98\x17\xf8\x16[\x81\xf2Y\xd9(\xce-\xdb\xfc\x9b\x02\x07\x0b\x87\xce\x95b\xa0U\xac\xbb\xdc\xf9~f\xbey!AA6\xd0\x8c^\xd2\xbf;\xa0H\xaf\xe6\xdc\xae\xba\xfe\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x00v y\xbef~\xf9\xdc\xbb\xacU\xa0b\x95\xce\x87\x0b\x07\x02\x9b\xfc\xdb-\xce(\xd9Y\xf2\x81[\x16\xf8\x17\x98v\x02\x02 |~R~|R|~kkkkk\xaaQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7f|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~\x01\x00~\x81l\x93l\x97vl}R\x96\xa0c|\x94guh\x82|\x01 \x80Q\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7f|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~\x01 {\x94\x7fw\x82v\x01$\x93\x010|~l~|~|~\x01A~l\xadd\x00\x00\x00\x00\x00\x00\x00\xff\xff\xff\xff]\xf6\xe0\xe2v\x13Y\xd3\n\x82u\x05\x8e)\x9f\xcc\x03\x81SEE\xf5\\\xf4>A\x98?]L\x94V\x00\x00\x00\x00A\x00\x00\x00'
        # Single sha
        sha256_preimage_sighash = b'\x01\xec\xe1eP\x8e6\x15\x99\x9f\xd2\xa1sei\x0b\x9b\x05\xf3a\xc7\x88\xba(\x0e\xc4\x08;\xba,\xbcJ'
        # Sighash
        sighash = b'\xab\x882\x03\xdb@0\x1f\x0fk\x85D\xbb\xbcE\xf8\xbe\xfd\x1b4\xc5;\xf0\xd8k9_\x87\x8f\xcb\xc4\xd4'

        tx = Tx(
            version=1,
            tx_ins=[dummy_tx_in],
            tx_outs=[],
            locktime=0
        )

        script_sig = Script()
        script_sig.append_pushdata(preimage_sighash)
        dummy_tx_in.script_sig = script_sig
        full_script = dummy_tx_in.script_sig + pushtx_lock

        #context = Context(script=dummy_tx_in.script_sig + pushtx_lock, z=sha256_preimage_sighash)
        context = Context(script=dummy_tx_in.script_sig + pushtx_lock, z=sighash)
        self.assertTrue(context.evaluate())

    def test_sig_hash_hardcoded_values_failure(self):
        txid = "413c9c771fa794d2b1b1e51a84347f859021b7a1c401701ffaced59c9e71eabd"
        prev_value = 100
        vout = 0
        pushtx_lock = pushtx(0x41,True,False,True)

        dummy_tx_in = TxIn(prev_tx=txid,prev_index=vout)

        # Preimage sighash for tx below computed with old tx_engine for SIGHASH_ALL, assuming that dummy_tx_in is locked with pushtx_lock
        preimage_sighash = b'\x01\x00\x00\x00\xddH\xb0\x86_|}\x1fT|H\n\x93p\x19f\xa9\xef\xf1Qt&\x04\xcaE\xcf\x0f\xe5h\x87j\xe5;\xb10)\xce{\x1fU\x9e\xf5\xe7G\xfc\xacC\x9f\x14U\xa2\xec|_\t\xb7"\x90y^pfPD\xbd\xeaq\x9e\x9c\xd5\xce\xfa\x1fp\x01\xc4\xa1\xb7!\x90\x85\x7f4\x84\x1a\xe5\xb1\xb1\xd2\x94\xa7\x1fw\x9c<A\x00\x00\x00\x00\xfd\xa2\x01 \x98\x17\xf8\x16[\x81\xf2Y\xd9(\xce-\xdb\xfc\x9b\x02\x07\x0b\x87\xce\x95b\xa0U\xac\xbb\xdc\xf9~f\xbey!AA6\xd0\x8c^\xd2\xbf;\xa0H\xaf\xe6\xdc\xae\xba\xfe\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x00v y\xbef~\xf9\xdc\xbb\xacU\xa0b\x95\xce\x87\x0b\x07\x02\x9b\xfc\xdb-\xce(\xd9Y\xf2\x81[\x16\xf8\x17\x98v\x02\x02 |~R~|R|~kkkkk\xaaQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7f|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~\x01\x00~\x81l\x93l\x97vl}R\x96\xa0c|\x94guh\x82|\x01 \x80Q\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7fQ\x7f|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~|~\x01 {\x94\x7fw\x82v\x01$\x93\x010|~l~|~|~\x01A~l\xadd\x00\x00\x00\x00\x00\x00\x00\xff\xff\xff\xff]\xf6\xe0\xe2v\x13Y\xd3\n\x82u\x05\x8e)\x9f\xcc\x03\x81SEE\xf5\\\xf4>A\x98?]L\x94V\x00\x00\x00\x00A\x00\x00\x00'
        # Single sha
        sha256_preimage_sighash = b'\x01\xec\xe1eP\x8e6\x15\x99\x9f\xd2\xa1sei\x0b\x9b\x05\xf3a\xc7\x88\xba(\x0e\xc4\x08;\xba,\xbcJ'
        # Sighash
        sighash = b'\xab\x882\x03\xdb@0\x1f\x0fk\x85D\xbb\xbcE\xf8\xbe\xfd\x1b4\xc5;\xf0\xd8k9_\x87\x8f\xcb\xc4\xd4'

        tx = Tx(
            version=1,
            tx_ins=[dummy_tx_in],
            tx_outs=[],
            locktime=0
        )

        script_sig = Script()
        script_sig.append_pushdata(preimage_sighash)
        dummy_tx_in.script_sig = script_sig
        full_script = dummy_tx_in.script_sig + pushtx_lock

        context = Context(script=dummy_tx_in.script_sig + pushtx_lock, z=sha256_preimage_sighash)
        self.assertFalse(context.evaluate())


if __name__ == "__main__":
    unittest.main()