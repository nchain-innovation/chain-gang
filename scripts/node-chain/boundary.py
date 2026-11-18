from harness import *
assert rpc('getblockcount')<180
parents={name:fund(lock) for name,lock in [('bip',P2PK),('otda',P2PK),('oldmul',Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL')),('oldnop',Script.parse_string('OP_LEFT OP_1')),('comp63',locking_script(OWNER,version=8)),('compe3',locking_script(OWNER,version=8)),('fee63',TRUE),('feee3',TRUE)]}
heights={k:rpc('getblock',rpc('getrawtransaction',p.id(),True)['blockhash'])['height'] for k,p in parents.items()}
(OUT/'boundary-parents.json').write_text(json.dumps({'parents':{k:raw(p) for k,p in parents.items()},'heights':heights},indent=2))
rpc('generate',198-rpc('getblockcount'))
otda=k1_spend(parents['otda'],0x61)
admission('pre-activation-61',otda,[parents['otda']],'rejected',[heights['otda']]);block_check('pre-activation-61-block',[otda],False)
for f in [0x63,0xe3]:
 suffix=f'{f:x}';ps=[parents['fee'+suffix],parents['comp'+suffix]]
 t=p256_spend(ps,f)
 admission('pre-activation-companion-'+suffix,t,ps,'rejected',[heights['fee'+suffix],heights['comp'+suffix]])
 block_check('pre-activation-companion-'+suffix+'-block',[t],False)
bip=k1_spend(parents['bip'],0x41)
admission('pre-activation-41-control',bip,[parents['bip']],'regular',[heights['bip']]);block_check('height-199-control',[bip],True)
# Exactly at the activation candidate height; output era is still old.
admission('activation-61-old-utxo',otda,[parents['otda']],'regular',[heights['otda']])
newmul=fund(Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL'),mine=False)
block_check('height-200-activation',[otda,newmul],True)
old=tx_for([parents['oldmul']],version=1)
admission('post-activation-old-opcode',old,[parents['oldmul']],'rejected',[heights['oldmul']]);block_check('old-opcode-block',[old],False)
new=tx_for([newmul],version=1)
admission('post-activation-new-opcode-v1',new,[newmul],'regular',[200]);mine_check('new-opcode-height-201',new)
# Pre-Chronicle NOP is consensus-valid but policy-discouraged.
t=tx_for([parents['oldnop']],version=1)
admission('old-nop-policy',t,[parents['oldnop']],'rejected',[heights['oldnop']]);block_check('old-nop-consensus',[t],True)
for f in [0x63,0xe3]:
 suffix=f'{f:x}';ps=[parents['fee'+suffix],parents['comp'+suffix]]
 t=p256_spend(ps,f)
 admission('post-activation-companion-'+suffix,t,ps,'regular',[heights['fee'+suffix],heights['comp'+suffix]])
 mine_check('companion-'+suffix+'-mined',t)
