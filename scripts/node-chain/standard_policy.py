from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)
# Both the funding lock and spend outputs use standard P2PK; no nonstandard output exception.
for locktime in [0x7fffffff,0x80000000,0xffffffff]:
 p=fund(P2PK)
 t=k1_spend(p,locktime=locktime,out_script=P2PK)
 row=admission(f'standard-locktime-{locktime:x}',t,[p],'regular')
 mine_check(f'standard-locktime-{locktime:x}-mined',t)
# Same high timestamp is non-final with a non-final sequence, not malformed data.
p=fund(P2PK)
t=k1_spend(p,sequences=[1],locktime=0xffffffff,out_script=P2PK)
admission('standard-maximum-time-nonfinal',t,[p],'nonfinal');block_check('standard-maximum-time-nonfinal-block',[t],False)
final=k1_spend(p,sequences=[0xffffffff],locktime=0xffffffff,out_script=P2PK)
admission('standard-maximum-time-finalization',final,[p],'regular');mine_check('standard-maximum-time-finalization-mined',final)
# BIP113 is now active, so MTP equality is non-final for both paths.
p=fund(P2PK);mtp=rpc('getblockheader',rpc('getbestblockhash'))['mediantime']
t=k1_spend(p,sequences=[1],locktime=mtp,out_script=P2PK)
admission('standard-mtp-equality',t,[p],'nonfinal');block_check('standard-mtp-equality-block',[t],False)
final=k1_spend(p,locktime=mtp,out_script=P2PK)
admission('standard-mtp-finalization',final,[p],'regular');mine_check('standard-mtp-finalization-mined',final)
# A nonstandard output is genuinely rejected under this profile.
p=fund(P2PK);t=k1_spend(p)
admission('standard-profile-rejects-op-true-output',t,[p],'rejected');block_check('standard-profile-nonstandard-consensus',[t],True)
