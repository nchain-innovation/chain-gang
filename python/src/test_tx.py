
# from tx_engine.tx.tx import Tx, TxIn, TxOut

# from tx_engine.chain_gang import py_vin_serialize, PyTxIn, PyTxOut
from tx_engine.chain_gang import TxIn, TxOut, Tx
# from tx_engine.engine.script import Script
# from tx_engine.engine.op_codes import OP_0, OP_RETURN


def main():
    # vin = TxIn(prev_tx=bytes(32), prev_index=0,sig_script=b'', sequence=0xFFFFFFFF )
    v2 = TxIn(prev_tx=bytes(32), prev_index=0, sig_script=b'', sequence=0xFFFFFFFF, )
    # tx_out = TxOut(amount=0, script_pubkey=Script([OP_0, OP_RETURN, bytes.fromhex("000000000000")]))
    # testnet = self.blockchain_interface.isTestNet()
    # spending_tx = Tx(version=1, tx_ins=[vin], tx_outs=[tx_out], locktime=0)
    out = TxOut(amount=100, script_pubkey=b'')
    # retval = py_vin_serialize(v2)
    # retval = v2.serialise()
    # print(f"retval = {retval}")

    tx = Tx(version=1, tx_ins=[v2], tx_outs=[out], locktime=0)
    retval = tx.serialize()
    print(f"serialize() = {retval}, {type(retval)}")

    retval = tx.hash()
    print(f"hash = {retval}, {type(retval)}")

    retval = tx.id()
    print(f"id = {retval}, {type(retval)}")

    retval = tx.is_coinbase()
    print(f"is_coinbase = {retval}, {type(retval)}")



if __name__ == '__main__':
    main()
