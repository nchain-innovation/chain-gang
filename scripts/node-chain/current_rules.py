"""Post-Chronicle transactions and newly-created outputs only."""
from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)

def signing_lock(pub, multi=False, verify=False):
 lock=Script()
 if multi:lock.append_data(bytes([0x51]))
 lock.append_pushdata(pub)
 if multi:lock.append_data(bytes([0x51]))
 lock.append_data(bytes([0xaf if verify else 0xae] if multi else [0xad if verify else 0xac]))
 lock.append_data(bytes([0x51 if verify else 0x91]))
 return lock

def check_case(name,sig,pub=K1PUB,version=2,multi=False,verify=False,expected=True):
 p=fund(signing_lock(pub,multi,verify));t=tx_for([p],version=version)
 unlock=Script()
 if multi:unlock.append_data(bytes([0]))
 unlock.append_pushdata(sig);t.tx_ins=[TxIn(p.id(),0,unlock,0xffffffff)]
 r=admission(name,t,[p],'regular' if expected else 'rejected')
 assert r['library']['accepted']==expected,(name,r['library'])
 if expected:mine_check(name+'-mined',t)
 else:
  r=block_check(name+'-block', [t],False)
  assert r['submit_result']=='blk-bad-inputs',r

order=0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
scalar_cases={'zero-r':(0,1),'zero-s':(1,0),'order-r':(order,1),'order-s':(1,order),'wrong-math':(1,1)}
for label,(r,s) in scalar_cases.items():
 sig=utils.encode_dss_signature(r,s)+bytes([0x61])
 for multi in [False,True]:
  name=f'current-{label}-'+('multisig' if multi else 'checksig')
  check_case(name,sig,multi=multi)
  check_case(name+'-v1-nullfail',sig,version=1,multi=multi,expected=False)
  check_case(name+'-verify',sig,multi=multi,verify=True,expected=False)
for flag in [0x41,0x61,0x63,0xe3]:
 check_case(f'current-zero-r-type-{flag:02x}',utils.encode_dss_signature(0,1)+bytes([flag]))
for name,sig in [('negative-r','300602018002010161'),('padded-r','30070202000102010161'),('trailing-data','30060201010201010061'),('undefined-type','300602010102010160'),('missing-forkid','300602010102010121')]:
 check_case('current-bad-encoding-'+name,bytes.fromhex(sig),expected=False)
for label,pub,syntax_ok in [('off-curve',bytes([2])+bytes([255])*32,True),('short',bytes([2]),False),('hybrid',bytes([6])+bytes(64),False)]:
 for empty in [False,True]:
  check_case(f'current-pubkey-{label}-empty-{empty}',b'' if empty else utils.encode_dss_signature(1,1)+bytes([0x61]),pub=pub,expected=syntax_ok)
check_case('current-empty-valid-key-v1',b'',version=1)
# Signature failure and transaction finality are separate: an OP_NOT script
# can pass but must still wait when nLockTime has not elapsed.
p=fund(signing_lock(K1PUB));lock=rpc('getblockcount')+1
sig=utils.encode_dss_signature(0,1)+bytes([0x61]);u=Script();u.append_pushdata(sig)
t=tx_for([p],sequences=[1],locktime=lock);t.tx_ins=[TxIn(p.id(),0,u,1)]
admission('current-false-signature-script-nonfinal',t,[p],'nonfinal')
r=block_check('current-false-signature-script-unelapsed',[t],False);assert r['submit_result']=='bad-txns-nonfinal'
block_check('current-false-signature-empty-advance',[],True)
block_check('current-false-signature-matured',[t],True)
# All six OTDA types: real full companion spends, two inputs/two outputs.
for flag in [0x61,0x62,0x63,0xe1,0xe2,0xe3]:
 ps=[fund(TRUE),fund(locking_script(OWNER,version=8))];future=rpc('getblockcount')+1000
 base=p256_spend(ps,flag,[4_000_000,4_000_000],sequences=[10,10],locktime=future)
 admission(f'current-otda-{flag:02x}-nonfinal',base,ps,'nonfinal')
 bad=parse(raw(base));bad.locktime=future+1
 r=admission(f'current-otda-{flag:02x}-stale-locktime',bad,ps,'rejected');assert not r['library']['accepted']
 other=parse(raw(base));ins=list(other.tx_ins);ins[0]=TxIn(ps[0].id(),0,ins[0].script_sig,11);other.tx_ins=ins
 expected='rejected' if flag==0x61 else 'nonfinal'
 r=admission(f'current-otda-{flag:02x}-other-sequence',other,ps,expected);assert r['library']['accepted']==(expected=='nonfinal')
 updated=p256_spend(ps,flag,[4_000_000,4_000_000],sequences=[12,11],locktime=future+1)
 admission(f'current-otda-{flag:02x}-resigned-update',updated,ps,'nonfinal')
 final=p256_spend(ps,flag,[4_000_000,4_000_000],sequences=[0xffffffff,0xffffffff],locktime=future+1)
 admission(f'current-otda-{flag:02x}-final',final,ps,'regular');mine_check(f'current-otda-{flag:02x}-mined',final)
log({'name':'current-rules-finished','result':'passed'})
