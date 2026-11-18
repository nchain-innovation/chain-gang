"""CLTV/CSV remain active for pre-Genesis outputs after Chronicle."""
from harness import *
assert rpc('getblockcount')==0
rpc('generate',110)
# regtest BIP65 is 1351, CSV is 576; spend above both, Genesis override 150.
scripts={
 'high-cltv':Script.parse_string('2147483648 OP_CHECKLOCKTIMEVERIFY OP_DROP OP_1'),
 'wide-csv':Script.parse_string('4294967297 OP_CHECKSEQUENCEVERIFY OP_DROP OP_1'),
 'disabled-csv':Script.parse_string('2147483648 OP_CHECKSEQUENCEVERIFY OP_DROP OP_1'),
 'time-csv':Script.parse_string('4194305 OP_CHECKSEQUENCEVERIFY OP_DROP OP_1'),
 'cltv':Script.parse_string('OP_1 OP_CHECKLOCKTIMEVERIFY OP_DROP OP_1'),
 'csv':Script.parse_string('OP_1 OP_CHECKSEQUENCEVERIFY OP_DROP OP_1'),
 'negative-cltv':Script.parse_string('OP_1NEGATE OP_CHECKLOCKTIMEVERIFY OP_DROP OP_1'),
 'negative-csv':Script.parse_string('OP_1NEGATE OP_CHECKSEQUENCEVERIFY OP_DROP OP_1'),
}
parents={name:fund(script) for name,script in scripts.items()}
heights={name:rpc('getblock',rpc('getrawtransaction',p.id(),True)['blockhash'])['height'] for name,p in parents.items()}
rpc('generate',1400-rpc('getblockcount'))
for name in ['cltv','csv']:
 p=parents[name];h=heights[name];old=[(p.id(),0)]
 for suffix,seq,lock in [('insufficient',0,0),('final-sequence',0xffffffff,1)]:
  t=tx_for([p],sequences=[seq],locktime=lock)
  r=admission(f'legacy-{name}-{suffix}',t,[p],'rejected',[h],old);assert not r['library']['accepted']
  r=block_check(f'legacy-{name}-{suffix}-block',[t],False);assert r['submit_result']=='blk-bad-inputs'
 t=tx_for([p],sequences=[1],locktime=1)
 admission(f'legacy-{name}-satisfied',t,[p],'regular',[h],old);mine_check(f'legacy-{name}-satisfied-mined',t)
for name in ['negative-cltv','negative-csv']:
 p=parents[name];t=tx_for([p],sequences=[1],locktime=1)
 r=admission('legacy-'+name,t,[p],'rejected',[heights[name]],[(p.id(),0)]);assert not r['library']['accepted']
 block_check('legacy-'+name+'-block',[t],False)
 # Same script on a new output: opcode is a policy-discouraged NOP, so
 # even the negative operand no longer has lock semantics in consensus.
 p=fund(scripts[name]);h=rpc('getblockcount');t=tx_for([p],sequences=[0],locktime=0)
 r=admission('new-'+name+'-policy',t,[p],'rejected',[h]);assert r['library']['accepted'];assert 'NOP' in r['rpc_error']
 block_check('new-'+name+'-consensus',[t],True)
# Five-byte operands and CSV's distinct type/disable/version branches.
p=parents['high-cltv'];h=heights['high-cltv'];old=[(p.id(),0)]
for lock in [0x7fffffff,0x80000000]:
 t=tx_for([p],sequences=[1],locktime=lock)
 if lock==0x7fffffff:
  r=admission('legacy-high-cltv-below',t,[p],'rejected',[h],old);assert not r['library']['accepted']
 else:
  admission('legacy-high-cltv-equal',t,[p],'nonfinal',[h],old)
  r=block_check('legacy-high-cltv-script-valid-not-final',[t],False);assert r['submit_result']=='bad-txns-nonfinal'
for name,goodseq in [('wide-csv',1),('time-csv',0x400001)]:
 p=parents[name];h=heights[name];old=[(p.id(),0)]
 for suffix,seq,version in [('wrong-type',0x400001 if goodseq==1 else 1,2),('version-one',goodseq,1),('disabled-input',0x80000001,2)]:
  t=tx_for([p],sequences=[seq],version=version)
  r=admission('legacy-'+name+'-'+suffix,t,[p],'rejected',[h],old);assert not r['library']['accepted']
  block_check('legacy-'+name+'-'+suffix+'-block',[t],False)
 t=tx_for([p],sequences=[goodseq]);admission('legacy-'+name+'-satisfied',t,[p],'regular',[h],old);mine_check('legacy-'+name+'-mined',t)
p=parents['disabled-csv'];t=tx_for([p],sequences=[0],version=1)
admission('legacy-csv-disabled-operand-v1',t,[p],'regular',[heights['disabled-csv']],[(p.id(),0)]);mine_check('legacy-csv-disabled-operand-mined',t)
log({'name':'legacy-locks-finished','result':'passed'})
