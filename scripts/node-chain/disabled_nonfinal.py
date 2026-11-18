from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)
p=fund(TRUE);lock=rpc('getblockcount')+1
t=tx_for([p],sequences=[1],locktime=lock)
r=admission('disabled-pool-nonfinal',t,[p],'rejected');assert 'non-final-pool-full' in r['rpc_error']
r=block_check('disabled-pool-unelapsed-block',[t],False);assert r['submit_result']=='bad-txns-nonfinal'
block_check('disabled-pool-empty-advance',[],True)
admission('disabled-pool-now-final',t,[p],'regular');mine_check('disabled-pool-final-mined',t)
