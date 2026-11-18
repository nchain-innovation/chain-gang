from harness import *
assert rpc('getblockcount')>=200
for flag in [0x41,0x42,0x43,0xc1,0xc2,0xc3,0x61,0x62,0x63,0xe1,0xe2,0xe3]:
 p=fund(locking_script(OWNER,version=8))
 t=p256_spend([p],flag,[4_000_000])
 bad=parse(raw(t));bad.tx_ins=[TxIn(p.id(),0,t.tx_ins[0].script_sig,0xfffffffe)]
 admission(f'paired-full-{flag:02x}-bad-sequence',bad,[p],'rejected');block_check(f'paired-full-{flag:02x}-bad-sequence-block',[bad],False)
 admission(f'paired-full-{flag:02x}',t,[p],'regular');mine_check(f'paired-full-{flag:02x}-mined',t)
for flag in [0x63,0xe3]:
 fee=fund(TRUE);p=fund(locking_script(OWNER,version=8));other=fund(locking_script(OWNER,version=8));ps=[fee,p]
 base=p256_spend(ps,flag)
 mutations=[]
 t=parse(raw(base));t.tx_outs=list(t.tx_outs)+[TxOut(1,TRUE)];mutations.append(('paired-output-added',t,ps))
 t=parse(raw(base));t.locktime=1;mutations.append(('locktime',t,ps))
 t=parse(raw(base));ins=list(t.tx_ins);ins[1]=TxIn(p.id(),0,ins[1].script_sig,0xfffffffe);t.tx_ins=ins;mutations.append(('own-sequence',t,ps))
 t=parse(raw(base));ins=list(t.tx_ins);ins[1]=TxIn(other.id(),0,ins[1].script_sig,ins[1].sequence);t.tx_ins=ins;mutations.append(('parent-swapped',t,[fee,other]))
 unsigned=tx_for(ps);a=authorization(unsigned,1,p.tx_outs[0].amount,flag);sig=sign(a,KEY);bad=sig[:-1]+bytes([sig[-1]^1]);w=universal_unlocking_script(p.serialize(),a,PUB,bad)
 t=Tx(2,[TxIn(fee.id(),0,Script()),TxIn(p.id(),0,w)],unsigned.tx_outs,0);mutations.append(('bad-r1-witness',t,ps))
 for label,t,parents in mutations:
  admission(f'companion-{flag:02x}-{label}',t,parents,'rejected');block_check(f'companion-{flag:02x}-{label}-block',[t],False)
 # SINGLE's companion does not commit to the other input's sequence.
 t=parse(raw(base));ins=list(t.tx_ins);ins[0]=TxIn(fee.id(),0,ins[0].script_sig,1);t.tx_ins=ins
 admission(f'companion-{flag:02x}-other-sequence',t,ps,'regular');mine_check(f'companion-{flag:02x}-other-sequence-mined',t)
# Default relay fee is zero; mining fee is a separate configured threshold.
p=fund(TRUE);t=tx_for([p],[p.tx_outs[0].amount]);admission('zero-fee-secondary',t,[p],'regular')
h=rpc('generate',1)[0];assert t.id() not in rpc('getblock',h)['tx'];assert t.id() in pools()['regular']
log({'name':'zero-fee-not-selected','mempool':'regular','entry':rpc('getmempoolentry',t.id()),'mined_block':h,'included':False,'mempoolinfo':rpc('getmempoolinfo')})
block_check('zero-fee-consensus',[t],True)
