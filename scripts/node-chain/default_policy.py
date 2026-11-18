from harness import *
assert rpc('getblockcount')==0
rpc('generate',600)
for flag in [0x63,0xe3]:
 ps=[fund(TRUE),fund(locking_script(OWNER,version=8))]
 t=p256_spend(ps,flag)
 admission(f'default-policy-companion-{flag:02x}',t,ps,'regular');mine_check(f'default-policy-companion-{flag:02x}-mined',t)
