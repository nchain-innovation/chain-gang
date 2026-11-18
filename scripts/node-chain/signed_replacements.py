from harness import *
for flag in [0x63,0xe3]:
 ps=[fund(TRUE),fund(locking_script(OWNER,version=8))];future=rpc('getblockcount')+1000
 base=p256_spend(ps,flag,sequences=[10,10],locktime=future)
 admission(f'signed-{flag:02x}-nonfinal',base,ps,'nonfinal')
 # This update satisfies monotonicity but invalidates the companion signature.
 bad=parse(raw(base));ins=list(bad.tx_ins);ins[1]=TxIn(ps[1].id(),0,ins[1].script_sig,11);bad.tx_ins=ins
 admission(f'signed-{flag:02x}-own-sequence-without-resigning',bad,ps,'rejected');block_check(f'signed-{flag:02x}-invalid-update-block',[bad],False)
 # Another input's sequence is uncommitted by SINGLE, but pool monotonicity still applies.
 down=parse(raw(base));ins=list(down.tx_ins);ins[0]=TxIn(ps[0].id(),0,ins[0].script_sig,9);down.tx_ins=ins
 row=admission(f'signed-{flag:02x}-other-sequence-down',down,ps,'rejected');assert row['library']['accepted'];assert 'bad-txn-update' in row['rpc_error']
 up=parse(raw(base));ins=list(up.tx_ins);ins[0]=TxIn(ps[0].id(),0,ins[0].script_sig,11);up.tx_ins=ins
 admission(f'signed-{flag:02x}-other-sequence-up',up,ps,'nonfinal');assert base.id() not in pools()['nonfinal']
 resigned=p256_spend(ps,flag,sequences=[12,11],locktime=future)
 admission(f'signed-{flag:02x}-resigned-update',resigned,ps,'nonfinal')
 final=p256_spend(ps,flag,sequences=[0xffffffff,0xffffffff],locktime=future)
 admission(f'signed-{flag:02x}-resigned-final',final,ps,'regular');mine_check(f'signed-{flag:02x}-final-mined',final)
