from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)
p=fund(TRUE);future=rpc('getblockcount')+1000
previous=None
for seq in [1,2,3]:
 t=tx_for([p],sequences=[seq],locktime=future)
 admission(f'rate-sequence-{seq}',t,[p],'nonfinal')
 if previous:assert previous not in pools()['nonfinal']
 previous=t.id()
t=tx_for([p],sequences=[4],locktime=future)
r=admission('rate-third-update-rejected',t,[p],'rejected');assert 'non-final-txn-replacement-rate' in r['rpc_error'];assert previous in pools()['nonfinal']
# A policy update throttle cannot make a final transaction consensus-invalid.
final=tx_for([p],sequences=[0xffffffff],locktime=future)
r=admission('rate-finalization-still-throttled',final,[p],'rejected');assert 'non-final-txn-replacement-rate' in r['rpc_error'];assert previous in pools()['nonfinal']
block_check('rate-throttle-does-not-invalidate-final-block',[final],True)
remaining=pools()
assert remaining['regular']==[] and remaining['nonfinal']==[previous]
assert rpc('gettxout',p.id(),0,False) is None
log({'name':'rate-stale-nonfinal-entry-after-confirmed-conflict','result':'observed','pools':remaining,'spent_outpoint':p.id()+':0','utxo':None,'note':'v1.2.3 non-final pool retains this entry; chain UTXO state is authoritative, not pool membership.'})
