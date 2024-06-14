
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

    raw_tx = bytes.fromhex(
        "0100000001813f79011acb80925dfe69b3def355fe914bd1d96a3f5f71bf8303c6a989c7d1000000006b483045022100ed81ff192e75a3fd2304004dcadb746fa5e24c5031ccfcf21320b0277457c98f02207a986d955c6e0cb35d446a89d3f56100f4d7f67801c31967743a9c8e10615bed01210349fc4e631e3624a545de3f89f5d8684c7b8138bd94bdd531d2e213bf016b278afeffffff02a135ef01000000001976a914bc3b654dca7e56b04dca18f2566cdaf02e8d9ada88ac99c39800000000001976a9141c4bc762dd5423e332166702cb75f40df79fea1288ac19430600"
    )
    tx = Tx.parse(raw_tx)
    print(f"tx = {tx}")
    assert len(tx.tx_outs) == 2
    want: int = 32454049
    assert tx.tx_outs[0].amount == 32454049
    actual: bytes = bytes.fromhex("1976a914bc3b654dca7e56b04dca18f2566cdaf02e8d9ada88ac")
    print(f"   s1 = {tx.tx_outs[0].script_pubkey.serialize().hex()}")
    print(f"bytes = {actual.hex()}")
    assert tx.tx_outs[0].script_pubkey.serialize() == actual
    assert tx.tx_outs[1].amount == 10011545
    actual_pubkey: bytes = bytes.fromhex("1976a9141c4bc762dd5423e332166702cb75f40df79fea1288ac")
    assert tx.tx_outs[1].script_pubkey.serialize() == actual_pubkey




if __name__ == '__main__':
    main()
