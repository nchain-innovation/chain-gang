"""Current Chronicle: valid encoding / invalid ECDSA is false, not an abort."""
from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)
lock=Script();lock.append_pushdata(K1PUB);lock.append_data(bytes([0xac,0x91]))
p=fund(lock)
t=tx_for([p]);unlock=Script();unlock.append_pushdata(bytes.fromhex('300602010002010161'))
t.tx_ins=[TxIn(p.id(),0,unlock,0xffffffff)]
admission('chronicle-zero-r-checksig-not',t,[p],'regular')
mine_check('chronicle-zero-r-mined',t)
