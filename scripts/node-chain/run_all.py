"""Fresh isolated node only. Run phases sequentially; chain state is intentional."""
from harness import *
assert rpc('getblockcount')==0, 'Use a fresh task-owned regtest node'
log({'name':'environment','result':'started','node_version':rpc('getnetworkinfo')['subversion'],'network':rpc('getblockchaininfo')['chain'],'source':'6504a3aff65ba97c0f6c80962b033e35ecbfed4b','runtime':'Apple Container','activation':200,'genesis':1,'csv_bip113':576,'policy':'flexible app profile; separate strict-policy run required'})
rpc('generate',110)
import boundary
import complete_spends
import sequences
import signed_replacements
remaining=pools();assert remaining['regular']==[]
for txid in remaining['nonfinal']:
 stale=rpc('getrawtransaction',txid,True)
 assert all(rpc('gettxout',i['txid'],i['vout'],False) is None for i in stale['vin'])
log({'name':'finished','result':'passed','height':rpc('getblockcount'),'pools':remaining,'note':'Any retained non-final entries have spent inputs; no ordinary pending transactions remain.'})
