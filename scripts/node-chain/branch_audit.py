"""Owning-source branch probes; fresh audit-profile Apple Container only."""
from harness import *

def reject(name, tx, parents, reason, heights=None):
 r=admission(name,tx,parents,'rejected',heights)
 assert reason in (r['rpc_error'] or ''),r
 return r

def wait_pool(name,tx,kind,timeout=20):
 deadline=time.monotonic()+timeout
 while time.monotonic()<deadline:
  state=pools()
  if tx.id() in state[kind]:
   log({'name':name,'result':'passed','txid':tx.id(),'pools':state});return
  time.sleep(.2)
 raise AssertionError((name,pools()))

assert rpc('getblockcount')==0
rpc('generate',110)
# Same-block/mempool-created outputs must use candidate height, not tip height.
seed=fund(TRUE)
oldtrue=fund(TRUE)
oldmul=fund(Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL'))
rpc('generate',198-rpc('getblockcount'))
parent=tx_for([seed]);parent.tx_outs=[TxOut(parent.tx_outs[0].amount,Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL'))]
child=tx_for([parent],version=1)
admission('before-boundary-parent',parent,[seed],'regular',[111])
r=admission('before-boundary-unconfirmed-child',child,[parent],'rejected',[199]);assert not r['library']['accepted']
block_check('before-boundary-package-invalid',[parent,child],False)
# Leave parent unmined while candidate height changes to activation.
row=block_check('empty-block-199-clears-era-cache',[],True)
assert parent.id() not in pools()['regular']
admission('activation-parent-resubmitted',parent,[seed],'regular',[111])
admission('activation-unconfirmed-child',child,[parent],'regular',[200])
block_check('activation-child-before-parent-invalid',[child,parent],False)
active=block_check('activation-same-block-package',[parent,child],True)
# Roll back across the activation candidate boundary; no public-chain effects.
rpc('invalidateblock',row['hash']);assert rpc('getblockcount')==198
r=admission('rollback-child-rules-old',child,[parent],'rejected',[199]);assert not r['library']['accepted']
rpc('reconsiderblock',row['hash']);rpc('reconsiderblock',active['hash'])
assert rpc('getbestblockhash')==active['hash']
log({'name':'reactivation-restores-package-chain','result':'passed','height':200,'hash':active['hash']})
# Flags must be recomputed per input in a mixed-era transaction.
newmul=fund(Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL'));newheight=rpc('getblockcount')
mixed=tx_for([oldtrue,newmul],version=1)
admission('mixed-era-old-true-new-mul',mixed,[oldtrue,newmul],'regular',[112,newheight]);mine_check('mixed-era-valid-mined',mixed)
newtrue=fund(TRUE);newheight=rpc('getblockcount')
for order in [0,1]:
 ps=[oldmul,newtrue] if order==0 else [newtrue,oldmul]
 hs=[113,newheight] if order==0 else [newheight,113]
 t=tx_for(ps,version=1)
 r=admission(f'mixed-era-old-mul-order-{order}',t,ps,'rejected',hs);assert not r['library']['accepted']
 block_check(f'mixed-era-old-mul-order-{order}-block',[t],False)
# IsFinalTx's exact height/time selector, with non-final and final controls.
rpc('generate',600-rpc('getblockcount'))
for lock,expected in [(499999999,'nonfinal'),(500000000,'regular'),(500000001,'regular')]:
 p=fund(TRUE);t=tx_for([p],sequences=[0],locktime=lock)
 admission(f'locktime-threshold-{lock}',t,[p],expected)
 if expected=='nonfinal':
  r=block_check(f'locktime-threshold-{lock}-block',[t],False);assert r['submit_result']=='bad-txns-nonfinal'
  t=tx_for([p],locktime=lock);admission('height-threshold-final-override',t,[p],'regular')
 mine_check(f'locktime-threshold-{lock}-mined',t)
# A change in locktime alone is not a sequence update, even if now final.
p=fund(TRUE);future=rpc('getblockcount')+1000
old=tx_for([p],sequences=[10],locktime=future);admission('locktime-update-initial',old,[p],'nonfinal')
for name,lock,seq,reason in [('equal-sequence',future+1,10,'bad-txn-update'),('increase-lock',future+1,9,'bad-txn-update'),('zero-lock',0,11,'txn-mempool-conflict'),('elapsed-lock',1,11,'txn-mempool-conflict')]:
 reject('locktime-update-'+name,tx_for([p],sequences=[seq],locktime=lock),[p],reason)
 assert old.id() in pools()['nonfinal']
new=tx_for([p],sequences=[11],locktime=future+1);admission('locktime-and-sequence-update',new,[p],'nonfinal')
final=tx_for([p],locktime=0);admission('locktime-update-all-final',final,[p],'regular');mine_check('locktime-update-final-mined',final)
# Cross the unsigned sequence high bit: these are monotone uint32 comparisons.
p=fund(TRUE);old=tx_for([p],sequences=[0x7fffffff],locktime=future);admission('sequence-high-bit-before',old,[p],'nonfinal')
new=tx_for([p],sequences=[0x80000000],locktime=future);admission('sequence-high-bit-forward',new,[p],'nonfinal')
reject('sequence-high-bit-backward',tx_for([p],sequences=[0x7fffffff],locktime=future+1),[p],'bad-txn-update')
final=tx_for([p],locktime=future);admission('sequence-high-bit-finalized',final,[p],'regular');mine_check('sequence-high-bit-final-mined',final)
# Final-transaction fast path is not permission to drop/reorder/add inputs.
ps=[fund(TRUE) for _ in range(3)]
old=tx_for(ps[:2],sequences=[10,10],locktime=future);admission('final-shape-initial',old,ps[:2],'nonfinal')
for name,parents,reason in [('subset',ps[:1],'bad-txn-update'),('reorder',ps[1::-1],'bad-txn-update'),('superset',ps,'txn-mempool-conflict')]:
 reject('final-shape-'+name,tx_for(parents,locktime=future),parents,reason)
 assert old.id() in pools()['nonfinal']
final=tx_for(ps[:2],locktime=future);admission('final-shape-exact',final,ps[:2],'regular');mine_check('final-shape-exact-mined',final)
# Replacement of two separate non-final entries is a different branch.
ps=[fund(TRUE) for _ in range(2)]
olds=[]
for i,p in enumerate(ps):
 t=tx_for([p],sequences=[1],locktime=future);admission(f'multi-update-old-{i}',t,[p],'nonfinal');olds.append(t)
reject('multi-update-nonfinal',tx_for(ps,sequences=[2,2],locktime=future),ps,'bad-txn-update')
reject('multi-update-final',tx_for(ps,locktime=future),ps,'txn-mempool-conflict')
assert all(t.id() in pools()['nonfinal'] for t in olds)
for i,p in enumerate(ps):
 t=tx_for([p],locktime=future);admission(f'multi-update-separate-final-{i}',t,[p],'regular');mine_check(f'multi-update-separate-mined-{i}',t)
# A non-final child is forbidden; final child cannot see a non-final pool output.
p=fund(TRUE);old=tx_for([p],sequences=[1],locktime=future);admission('nonfinal-chain-parent',old,[p],'nonfinal')
reject('nonfinal-chain-child',tx_for([old],sequences=[1],locktime=future),[old],'too-long-non-final-chain')
reject('nonfinal-chain-final-child',tx_for([old]),[old],'Missing inputs')
final=tx_for([p],locktime=future);admission('nonfinal-chain-finalized-parent',final,[p],'regular')
child=tx_for([final]);admission('final-chain-child',child,[final],'regular')
block_check('final-chain-package-mined',[final,child],True)
# Time/height maturation must go through periodic revalidation, without resigning.
p=fund(TRUE);height=rpc('getblockcount')+1;t=tx_for([p],sequences=[1],locktime=height)
admission('periodic-height-equality',t,[p],'nonfinal')
r=block_check('periodic-height-equality-block',[t],False);assert r['submit_result']=='bad-txns-nonfinal'
block_check('periodic-height-empty-advance',[],True)
wait_pool('periodic-height-promoted',t,'regular');assert t.id() not in pools()['nonfinal'];mine_check('periodic-height-mined',t)
log({'name':'branch-audit-finished','result':'passed','pools':pools()})
