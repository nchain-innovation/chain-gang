from harness import *
assert rpc('getblockcount')>=200
ps=[fund(TRUE) for _ in range(3)]
future=rpc('getblockcount')+1000
base=tx_for(ps[:2],sequences=[10,10],locktime=future)
admission('nonfinal-initial',base,ps[:2],'nonfinal')
block_check('nonfinal-not-mineable',[base],False)
for name,parents,seq in [('equal',ps[:2],[10,10]),('decrease',ps[:2],[9,10]),('mixed-up-down',ps[:2],[11,9]),('reordered',ps[1::-1],[11,11]),('input-added',ps,[11,11,11]),('input-removed',ps[:1],[11])]:
 t=tx_for(parents,sequences=seq,locktime=future);t.tx_outs=[TxOut(t.tx_outs[0].amount-1000,TRUE)]
 admission('nonfinal-reject-'+name,t,parents,'rejected')
 assert base.id() in pools()['nonfinal']
updated=tx_for(ps[:2],sequences=[11,10],locktime=future)
admission('nonfinal-one-increased',updated,ps[:2],'nonfinal');assert base.id() not in pools()['nonfinal']
partial=tx_for(ps[:2],sequences=[0xffffffff,10],locktime=future)
admission('nonfinal-partial-finalization',partial,ps[:2],'nonfinal')
block_check('partial-not-mineable',[partial],False)
final=tx_for(ps[:2],sequences=[0xffffffff,0xffffffff],locktime=future)
admission('nonfinal-finalize',final,ps[:2],'regular');assert not pools()['nonfinal']
conflict=tx_for(ps[:2],[final.tx_outs[0].amount-10000],locktime=future)
admission('higher-fee-final-conflict',conflict,ps[:2],'rejected');assert final.id() in pools()['regular']
block_check('conflict-valid-in-block',[conflict],True);assert final.id() not in pools()['regular']
# A low sequence with locktime zero is final: it is not automatically an RBF signal.
p=fund(TRUE);original=tx_for([p],sequences=[1]);admission('low-sequence-locktime-zero',original,[p],'regular')
replacement=tx_for([p],[original.tx_outs[0].amount-10000],sequences=[2]);admission('low-sequence-fee-bump-not-rbf',replacement,[p],'rejected');mine_check('low-sequence-original-mined',original)
# All sequence bits: Genesis disables BIP68 transaction-relative locks.
for seq in [0,65535,0x00400001,0x80000000,0xfffffffe,0xffffffff]:
 p=fund(TRUE);t=tx_for([p],sequences=[seq]);admission(f'sequence-bits-{seq:08x}',t,[p],'regular');mine_check(f'sequence-bits-{seq:08x}-mined',t)
# Explicitly probe the full uint32 locktime range; local result is captured, not assumed.
for locktime in [0x80000000,0xffffffff]:
 p=fund(TRUE);t=tx_for([p],locktime=locktime);admission(f'locktime-{locktime:x}-final',t,[p],'regular');mine_check(f'locktime-{locktime:x}-final-mined',t)
# Exact block-height locktime boundary: locktime is the last invalid height.
p=fund(TRUE);height=rpc('getblockcount')+1
t=tx_for([p],sequences=[1],locktime=height)
admission('height-lock-equal',t,[p],'nonfinal');block_check('height-lock-equal-block',[t],False)
rpc('generate',1)
block_check('height-lock-next-block',[t],True)
# Regtest CSV/BIP113 activates at 576, independently of our Chronicle override.
p=fund(TRUE);mtp=rpc('getblockheader',rpc('getbestblockhash'))['mediantime']
t=tx_for([p],sequences=[1],locktime=mtp)
admission('pre-csv-time-equal-mtp',t,[p],'nonfinal')
block_check('pre-csv-block-uses-header-time',[t],True)
rpc('generate',576-rpc('getblockcount'))
# Repeat after CSV activation: the previous low-height probes alone cannot
# establish that Genesis, rather than inactive CSV, disables BIP68.
for seq in [65535,0x00400001,0x80000000,0xfffffffe]:
 p=fund(TRUE);t=tx_for([p],sequences=[seq]);admission(f'post-csv-sequence-{seq:08x}',t,[p],'regular');mine_check(f'post-csv-sequence-{seq:08x}-mined',t)
p=fund(TRUE);mtp=rpc('getblockheader',rpc('getbestblockhash'))['mediantime']
t=tx_for([p],sequences=[1],locktime=mtp)
admission('time-lock-equal-mtp',t,[p],'nonfinal');block_check('time-lock-equal-mtp-block',[t],False)
control=tx_for([p],sequences=[0xffffffff],locktime=mtp)
admission('time-lock-final-control',control,[p],'regular');mine_check('time-lock-final-control-mined',control)
p=fund(TRUE);mtp=rpc('getblockheader',rpc('getbestblockhash'))['mediantime']
t=tx_for([p],sequences=[1],locktime=mtp-1)
admission('time-lock-below-mtp',t,[p],'regular');mine_check('time-lock-below-mtp-mined',t)
# A maximum timestamp is valid syntax but remains non-final with a non-final input.
p=fund(TRUE);t=tx_for([p],sequences=[1],locktime=0xffffffff)
admission('maximum-time-nonfinal',t,[p],'nonfinal');block_check('maximum-time-nonfinal-block',[t],False)
control=tx_for([p],sequences=[0xffffffff],locktime=0xffffffff)
admission('maximum-time-finalized',control,[p],'regular');mine_check('maximum-time-finalized-mined',control)
