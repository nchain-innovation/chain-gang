import os,sys,json,subprocess,time,hashlib,struct,re
from pathlib import Path
from decimal import Decimal
LAB=Path(os.environ.get('BSV_SCRIPT_LAB',str(Path(__file__).resolve().parents[1]/'bsv-script-lab')))
assert subprocess.check_output(['git','-C',str(LAB),'rev-parse','HEAD'],text=True).strip()=='f8f184bb6a35139ce8d1971a6f514cffc666e4d3', 'Use the pinned bsv-script-lab commit'
sys.path.insert(0,str(LAB/'research/secure-enclave-p256'))
from tx_engine import Tx,TxIn,TxOut,Script,sig_hash
from test_native import signer
from test_universal import sign
from handwritten import locking_script
from universal import authorization,universal_unlocking_script
NODE=os.environ.get('BSV_CONTAINER','cg-chain-validation-20261119')
OUT=Path(os.environ.get('CHAIN_OUT',str(Path(__file__).parent/'results')));OUT.mkdir(parents=True,exist_ok=True)
RECORDS=json.loads((OUT/'results.json').read_text()) if (OUT/'results.json').exists() else []
class RPCError(Exception):
 def __init__(self,msg):self.message=msg;super().__init__(msg)
def rpc(method,*args):
 data='\n'.join(a if isinstance(a,str) else json.dumps(a,separators=(',',':')) for a in args)+('\n' if args else '')
 p=subprocess.run(['container','exec','-i',NODE,'bitcoin-cli','-regtest','-rpcuser=cgtest','-rpcpassword=local-regtest-only','-stdin',method],input=data,text=True,capture_output=True,timeout=120)
 if p.returncode:raise RPCError(p.stderr.strip() or p.stdout.strip())
 if not p.stdout.strip():return None
 try:return json.loads(p.stdout)
 except json.JSONDecodeError:return p.stdout.strip()
def raw(tx):return tx.serialize().hex()
def parse(s):return Tx.parse_hexstr(s)
def pools():return {'regular':rpc('getrawmempool'),'nonfinal':rpc('getrawnonfinalmempool')}
def log(row):
 RECORDS.append(row);(OUT/'results.json').write_text(json.dumps(RECORDS,indent=2));print(row['name'],row.get('mempool',row.get('block',row.get('result'))),flush=True)
def fund(lock,amount=5_000_000,mine=True):
 tx=Tx(1,[],[TxOut(amount,lock)],0)
 funded=rpc('fundrawtransaction',raw(tx),{'changePosition':1,'feeRate':0.00002})
 signed=rpc('signrawtransaction',funded['hex']);assert signed['complete']
 parent=parse(signed['hex']);assert parent.tx_outs[0].script_pubkey.raw_serialize()==lock.raw_serialize()
 assert rpc('sendrawtransaction',raw(parent))==parent.id()
 if mine:rpc('generate',1)
 return parent
TRUE=Script.parse_string('OP_1')
def tx_for(parents,amounts=None,sequences=None,locktime=0,version=2):
 seq=sequences or [0xffffffff]*len(parents)
 if amounts is None:amounts=[sum(p.tx_outs[0].amount for p in parents)-500_000]
 return Tx(version,[TxIn(p.id(),0,Script(),seq[i]) for i,p in enumerate(parents)],[TxOut(a,TRUE) for a in amounts],locktime)
def local(tx,parents,heights=None,pregenesis_outpoints=None):
 try:
  if heights is None:tx.validate(parents)
  else:tx.validate_with_chronicle_context(parents,rpc('getblockcount')+1,200,[(p.id(),0,h) for p,h in zip(parents,heights)],consensus_only=True,pregenesis_outpoints=pregenesis_outpoints)
  return {'accepted':True}
 except ValueError as e:return {'accepted':False,'error':str(e)}
def admission(name,tx,parents,expected,heights=None,pregenesis_outpoints=None):
 before=pools()
 try:result=rpc('sendrawtransaction',raw(tx));error=None
 except RPCError as e:result=None;error=e.message
 after=pools();kind='regular' if tx.id() in after['regular'] else 'nonfinal' if tx.id() in after['nonfinal'] else 'rejected'
 row={'name':name,'txid':tx.id(),'tx':raw(tx),'parents':[raw(p) for p in parents],'height':rpc('getblockcount')+1,'library':local(tx,parents,heights,pregenesis_outpoints),'mempool':kind,'rpc_result':result,'rpc_error':error,'before':before,'after':after,'expected':expected}
 log(row);assert kind==expected,(name,expected,kind,error)
 if kind in ('regular','nonfinal'):assert row['library']['accepted'], ('node accepts but library rejects',name,row['library'])
 if error and '16: mandatory-script-verify-flag-failed' in error:assert not row['library']['accepted'], ('mandatory script disagreement',name)
 return row
def dsha(x):return hashlib.sha256(hashlib.sha256(x).digest()).digest()
def varint(n):
 if n<253:return bytes([n])
 return b'\xfd'+struct.pack('<H',n) if n<=65535 else b'\xfe'+struct.pack('<I',n)
COUNTER=0
def candidate(txs):
 global COUNTER
 COUNTER+=1
 tip=rpc('getbestblockhash');header=rpc('getblockheader',tip);height=header['height']+1
 h=height.to_bytes((height.bit_length()+7)//8,'little')
 if h[-1]&128:h+=b'\0'
 coin_script=bytes([len(h)])+h+struct.pack('<I',COUNTER)
 coin=struct.pack('<i',1)+b'\x01'+bytes(32)+b'\xff'*4+varint(len(coin_script))+coin_script+b'\xff'*4+b'\x01'+struct.pack('<q',(50*100_000_000)>>(height//150))+b'\x01\x51'+bytes(4)
 bodies=[coin]+[bytes.fromhex(raw(tx)) for tx in txs]
 level=[dsha(t) for t in bodies]
 while len(level)>1:
  if len(level)%2:level.append(level[-1])
  level=[dsha(level[i]+level[i+1]) for i in range(0,len(level),2)]
 prefix=struct.pack('<i',0x20000000)+bytes.fromhex(tip)[::-1]+level[0]+struct.pack('<II',header['time']+1,0x207fffff)
 target=0x7fffff<<(8*(0x20-3))
 for nonce in range(100000):
  hdr=prefix+struct.pack('<I',nonce)
  if int.from_bytes(dsha(hdr),'little')<=target:break
 block=hdr+varint(len(bodies))+b''.join(bodies)
 return block.hex(),dsha(hdr)[::-1].hex(),tip,height
def block_check(name,txs,expected):
 block,bhash,old,height=candidate(txs)
 spent_before={f'{i.prev_tx}:{i.prev_index}':rpc('gettxout',i.prev_tx,i.prev_index,False) for tx in txs for i in tx.tx_ins}
 result=rpc('submitblock',block)
 tip=rpc('getbestblockhash');accepted=tip==bhash
 row={'name':name,'block':accepted,'expected':expected,'height':height,'hash':bhash,'old_tip':old,'new_tip':tip,'submit_result':result,'transactions':[t.id() for t in txs]}
 path=OUT/(name+'.block.hex');path.write_text(block);row['block_file']=path.name
 if accepted:
  actual=rpc('getblock',bhash)['tx'];assert all(t.id() in actual for t in txs)
  row['confirmed']={t.id():rpc('getrawtransaction',t.id(),True).get('confirmations',0) for t in txs}
 log(row);assert accepted==expected,(name,result,accepted)
 if not accepted:
  assert tip==old
  spent_after={f'{i.prev_tx}:{i.prev_index}':rpc('gettxout',i.prev_tx,i.prev_index,False) for tx in txs for i in tx.tx_ins}
  assert spent_after==spent_before, 'Rejected block changed UTXO state'
 else:
  assert all(rpc('gettxout',i.prev_tx,i.prev_index,False) is None for tx in txs for i in tx.tx_ins)
 return row
def mine_check(name,tx):
 hashes=rpc('generate',1);block=rpc('getblock',hashes[0]);assert tx.id() in block['tx'],(name,block)
 log({'name':name,'block':True,'hash':hashes[0],'height':block['height'],'txid':tx.id(),'confirmations':rpc('getrawtransaction',tx.id(),True)['confirmations']})
def p256_spend(parents,flag,amounts=None,sequences=None,locktime=0):
 tx=tx_for(parents,amounts,sequences,locktime)
 ix=len(parents)-1
 a=authorization(tx,ix,parents[ix].tx_outs[0].amount,flag)
 witness=universal_unlocking_script(parents[ix].serialize(),a,PUB,sign(a,KEY))
 ins=list(tx.tx_ins);ins[ix]=TxIn(parents[ix].id(),0,witness,ins[ix].sequence)
 return Tx(tx.version,ins,tx.tx_outs,tx.locktime)
KEY,PUB,OWNER=signer()
if __name__=='__main__':
 p=fund(TRUE)
 t=tx_for([p]);admission('smoke',t,[p],'regular');mine_check('smoke-mined',t)
 # A genuinely invalid script must fail both routes, while preserving its UTXO.
 p=fund(Script.parse_string('OP_0'))
 t=tx_for([p]);admission('false-script',t,[p],'rejected');block_check('false-script-block',[t],False)

from cryptography.hazmat.primitives import hashes,serialization
from cryptography.hazmat.primitives.asymmetric import ec,utils
K1=ec.derive_private_key(19,ec.SECP256K1())
K1PUB=K1.public_key().public_bytes(serialization.Encoding.X962,serialization.PublicFormat.CompressedPoint)
P2PK=Script();P2PK.append_pushdata(K1PUB);P2PK.append_data(bytes([0xac]))
def k1_spend(parent,flag=0x41,version=2,sequences=None,locktime=0,out_script=None):
 tx=tx_for([parent],sequences=sequences,locktime=locktime,version=version)
 if out_script is not None:tx.tx_outs=[TxOut(tx.tx_outs[0].amount,out_script)]
 digest=bytes(sig_hash(tx,0,parent.tx_outs[0].script_pubkey,parent.tx_outs[0].amount,flag))
 der=K1.sign(digest,ec.ECDSA(utils.Prehashed(hashes.SHA256()),deterministic_signing=True))
 r,s=utils.decode_dss_signature(der);n=0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
 sig=utils.encode_dss_signature(r,min(s,n-s))+bytes([flag]);unlock=Script();unlock.append_pushdata(sig)
 return Tx(version,[TxIn(parent.id(),0,unlock,tx.tx_ins[0].sequence)],tx.tx_outs,locktime)
