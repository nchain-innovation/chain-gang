"""Current multisig cleans every supplied signature before matching any key."""
from harness import *
if rpc('getblockcount')==0:rpc('generate',600)
# The malformed second signature is skipped if the first match fails early.
# It is still part of the node's scriptCode cleanup before verification.
lock=Script();lock.append_pushdata(b'\x30\x01');lock.append_data(bytes([0x75,0x52]));lock.append_pushdata(K1PUB);lock.append_pushdata(K1PUB);lock.append_data(bytes([0x52,0xae,0x91]))
p=fund(lock);t=tx_for([p])
# This is the code the unfixed library erroneously hashes: the raw two-byte signature was
# searched instead of the pushed signature (02 30 01), so nothing was removed.
sign_cleaned=os.environ.get('CLEANUP_SIGN_CLEANED')=='1'
wrong_code=Script();wrong_code.append_data(lock.raw_serialize()[3:] if sign_cleaned else lock.raw_serialize())
digest=bytes(sig_hash(t,0,wrong_code,p.tx_outs[0].amount,0x61))
der=K1.sign(digest,ec.ECDSA(utils.Prehashed(hashes.SHA256()),deterministic_signing=True));r,s=utils.decode_dss_signature(der);n=0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
u=Script();u.append_data(bytes([0]));u.append_pushdata(b'\x30\x01');u.append_pushdata(utils.encode_dss_signature(r,min(s,n-s))+b'\x61');t.tx_ins=[TxIn(p.id(),0,u,0xffffffff)]
r=admission('current-unvisited-signature-cleanup',t,[p],'rejected' if sign_cleaned else 'regular')
assert r['library']['accepted']==(not sign_cleaned),r
if sign_cleaned:block_check('current-cleaned-signature-reaches-malformed-entry',[t],False)
else:mine_check('current-unvisited-signature-cleanup-mined',t)
