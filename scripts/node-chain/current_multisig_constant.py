"""Unpaired OTDA SINGLE plus BIP143 in the same current CHECKMULTISIG."""
from harness import *
if rpc('getblockcount')==0:rpc('generate',600)
assert rpc('getblockcount')>=600

def S(b):s=Script();s.append_data(b);return s

def push(b):s=Script();s.append_pushdata(b);return s.raw_serialize()

def sign_digest(digest,flag):
 der=K1.sign(digest,ec.ECDSA(utils.Prehashed(hashes.SHA256()),deterministic_signing=True))
 r,s=utils.decode_dss_signature(der);n=0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
 return utils.encode_dss_signature(r,min(s,n-s))+bytes([flag])
for flag in [0x63,0xe3]:
 # The constant digest makes this signature independent of the locking script,
 # so it can also appear inside that script without circular construction.
 constant=sign_digest(b'\x01'+bytes(31),flag)
 prefix=push(constant)
 code=prefix+b'\x75\x52'+push(K1PUB)*2+b'\x52\xaf\x51'
 for drop_push in [False,True]:
  ps=[fund(TRUE),fund(S(code))];t=tx_for(ps)
  signed_code=code[len(prefix):] if drop_push else code
  digest=bytes(sig_hash(t,1,S(signed_code),ps[1].tx_outs[0].amount,0x41))
  bip=sign_digest(digest,0x41)
  t.tx_ins=[t.tx_ins[0],TxIn(ps[1].id(),0,S(b'\x00'+push(constant)+push(bip)),0xffffffff)]
  name=f'constant-multisig-{flag:02x}-delete-forkid-push-{drop_push}'
  r=admission(name,t,ps,'rejected' if drop_push else 'regular');assert r['library']['accepted']==(not drop_push)
  r['library_consensus']=r['library']['accepted']
  if drop_push:
   b=block_check(name+'-block',[t],False);assert b['submit_result']=='blk-bad-inputs'
  else:mine_check(name+'-mined',t)
log({'name':'current-multisig-constant-finished','result':'passed'})
