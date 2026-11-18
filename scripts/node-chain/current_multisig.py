"""Current CHECKSIG/CHECKMULTISIG matching and scriptCode interactions."""
from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)

def S(data):
 s=Script();s.append_data(data);return s

def push(data):
 s=Script();s.append_pushdata(data);return s.raw_serialize()

def num(n):return Script.parse_string(str(n)).raw_serialize()

def signature(tx,parent,code,flag=0x61):
 digest=bytes(sig_hash(tx,0,S(code),parent.tx_outs[0].amount,flag))
 der=K1.sign(digest,ec.ECDSA(utils.Prehashed(hashes.SHA256()),deterministic_signing=True))
 r,s=utils.decode_dss_signature(der);n=0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
 return utils.encode_dss_signature(r,min(s,n-s))+bytes([flag])

def check(name,tx,parent,expected,consensus=None):
 if consensus is None:consensus=expected
 r=admission(name,tx,[parent],'regular' if expected else 'rejected')
 assert r['library']['accepted']==expected,(name,r['library'])
 try:
  tx.validate_with_chronicle_context([parent],rpc('getblockcount')+1,200,[(parent.id(),0,rpc('getblockcount'))],consensus_only=True)
  lib=True
 except ValueError:lib=False
 r['library_consensus']=lib;assert lib==consensus,(name,lib,consensus)
 if expected:mine_check(name+'-mined',tx)
 else:
  b=block_check(name+'-block',[tx],consensus)
  if not consensus:assert b['submit_result']=='blk-bad-inputs',b

# Compare exactly matching and nonmatching first signatures. The later entry
# is malformed (or empty with malformed key), and must be checked only if reached.
def cleanup_case(flag,empty,phase,form,match):
 bad=b'' if empty else (b'\x30'*75+b'\x01' if form=='long' else b'\x30\x01');pushed=push(bad)
 if form in ['canonical','long']:prefix=pushed+b'\x75';actual=b'\x75';other=prefix
 elif form=='repeat':prefix=(pushed+b'\x75')*2;actual=b'\x75\x75';other=prefix
 elif form=='alias':prefix=b'\x4c'+bytes([len(bad)])+bad+b'\x75';actual=prefix;other=b'\x75'
 elif form=='nested':prefix=push(b'\xaa'+pushed)+b'\x75';actual=prefix;other=b'\x75'
 elif form=='skipped':prefix=b'\x00\x63'+pushed+b'\x68';actual=b'\x00\x63\x68';other=prefix
 else:raise AssertionError(form)
 body=b'\x52'+push(b'\x02' if empty else K1PUB)+push(K1PUB)+b'\x52\xae\x91'
 evaluated=prefix+body;lock=evaluated if phase=='lock' else b'\x69\x51'
 parent=fund(S(lock));t=tx_for([parent])
 code=(actual if match else other)+body+(lock if phase=='unlock' else b'')
 sig=signature(t,parent,code,flag)
 u=b'\x00'+push(bad)+push(sig)
 if phase=='unlock':u+=b'\xab'+evaluated
 t.tx_ins=[TxIn(parent.id(),0,S(u),0xffffffff)]
 check(f'cleanup-{flag:02x}-{empty}-{phase}-{form}-match-{match}',t,parent,not match)
for flag in [0x41,0x61,0xe3]:
 for empty in [False,True]:
  for phase in ['lock','unlock']:
   for match in [False,True]:cleanup_case(flag,empty,phase,'canonical',match)
for flag in [0x41,0x61]:
 for form in ['repeat','alias','nested','skipped','long']:
  for match in [False,True]:cleanup_case(flag,False,'lock',form,match)

# Key matching must be lazy and ordered; zero signatures checks no key encoding.
def multi_case(name,keys,required,sigs=None,expected=True,version=2,dummy=b'',verify=False,invert=False,consensus=None,countbytes=None):
 code=num(required)+b''.join(push(k) for k in keys)+(num(len(keys)) if countbytes is None else countbytes)+bytes([0xaf if verify else 0xae])+bytes([0x91 if invert else 0x51 if verify else 0x61])
 p=fund(S(code));t=tx_for([p],version=version)
 if sigs is None:sigs=[signature(t,p,code)]
 u=(b'' if dummy is None else push(dummy))+b''.join(push(s) for s in sigs)
 t.tx_ins=[TxIn(p.id(),0,S(u),0xffffffff)]
 check(name,t,p,expected,consensus)
bad=b'\x30\x01';zero=utils.encode_dss_signature(0,1)+b'\x61'
multi_case('matching-unused-malformed-key',[b'\x02',K1PUB],1)
multi_case('matching-visited-malformed-key',[K1PUB,b'\x02'],1,expected=False)
multi_case('matching-zero-of-invalid-keys',[b'\x02',b'\x06'],0,[])
multi_case('matching-zero-of-zero',[],0,[])
multi_case('matching-zero-missing-dummy',[],0,[],dummy=None,expected=False)
multi_case('matching-too-many-signatures',[K1PUB],2,[b'',b''],expected=False)
multi_case('matching-negative-signature-count',[K1PUB],-1,[],expected=False)
multi_case('matching-five-byte-key-count',[K1PUB],1,[],expected=False,countbytes=push(b'\x01\x00\x00\x00\x00')+b'\x61')
multi_case('matching-early-exit-skips-malformed',[K1PUB,K1PUB],2,[bad,zero],invert=True)
multi_case('matching-visited-malformed-signature',[K1PUB,K1PUB],2,[zero,bad],invert=True,expected=False)
multi_case('matching-nullfail-includes-unvisited',[K1PUB,K1PUB],2,[bad,b''],invert=True,version=1,expected=False)
multi_case('matching-all-empty-v1',[K1PUB,K1PUB],2,[b'',b''],invert=True,version=1)
multi_case('matching-verify-cannot-invert-failure',[K1PUB,K1PUB],2,[bad,zero],verify=True,expected=False)
multi_case('matching-current-nonempty-dummy',[K1PUB],1,dummy=b'\x01')
multi_case('matching-v1-dummy-policy',[K1PUB],1,dummy=b'\x01',version=1,expected=False,consensus=True)
multi_case('matching-current-nonminimal-count',[K1PUB],1,countbytes=push(b'\x01\x00')+b'\x61')
multi_case('matching-v1-nonminimal-count-policy',[K1PUB],1,countbytes=push(b'\x01\x00')+b'\x61',version=1,expected=False,consensus=True)
# Actual valid mixed-type multisig must still match every required signature.
for flags in [(0x41,0x61),(0x61,0x41),(0x43,0xe3),(0xe1,0x62)]:
 code=b'\x52'+push(K1PUB)*2+b'\x52\xaf\x51'
 p=fund(S(code));t=tx_for([p]);sigs=[signature(t,p,code,f) for f in flags]
 t.tx_ins=[TxIn(p.id(),0,S(b'\x00'+b''.join(push(s) for s in sigs)),0xffffffff)]
 check(f'mixed-types-{flags[0]:02x}-{flags[1]:02x}',t,p,True)
# Both signature opcodes in one transaction, across the unlock/lock phase
# boundary. An executed separator excludes signature pushes from the unlock
# phase's scriptCode; that code still appends the entire locking script.
def corrupt(sig):
 r,s=utils.decode_dss_signature(sig[:-1]);return utils.encode_dss_signature(r+1,s)+sig[-1:]
for order in ['multi-unlock','single-unlock']:
 for csflag,msflag in [(0x61,0x41),(0x41,0xe3)]:
  for mutation in ['none','first','second']:
   if order=='multi-unlock':
    body=b'\x51'+push(K1PUB)+b'\x51\xaf';lock=push(K1PUB)+b'\xad\x51'
   else:
    body=push(K1PUB)+b'\xad';lock=b'\x51'+push(K1PUB)+b'\x51\xaf\x51'
   p=fund(S(lock));t=tx_for([p])
   cs=signature(t,p,lock if order=='multi-unlock' else body+lock,csflag)
   ms=signature(t,p,body+lock if order=='multi-unlock' else lock,msflag)
   if mutation!='none':
    change_ms=(mutation=='first')==(order=='multi-unlock')
    if change_ms:ms=corrupt(ms)
    else:cs=corrupt(cs)
   u=push(cs)+b'\x00'+push(ms) if order=='multi-unlock' else b'\x00'+push(ms)+push(cs)
   u+=b'\xab'+body;t.tx_ins=[TxIn(p.id(),0,S(u),0xffffffff)]
   check(f'cross-{order}-{csflag:02x}-{msflag:02x}-{mutation}',t,p,mutation=='none')
log({'name':'current-multisig-finished','result':'passed'})
