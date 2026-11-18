import sys,json,subprocess
from pathlib import Path
import argparse, os
parser=argparse.ArgumentParser(description="Replay pinned bsv-script-lab universal spends on Rust and BSV Node via Apple Container")
parser.add_argument("mode", choices=["baseline","fixed"])
parser.add_argument("output")
parser.add_argument("--lab",required=True, help="Checkout of linzhm/bsv-script-lab at f8f184bb6a35139ce8d1971a6f514cffc666e4d3")
args=parser.parse_args()
sys.path.insert(0,str(Path(args.lab)/'research/secure-enclave-p256'))
import test_universal as t
records=[]
def check(tx,parents,ix,valid=True):
    out=parents[ix].tx_outs[0]
    flags=sum(1<<b for b in (1,2,3,16,18,19,20,21))
    request=dict(tx=tx.serialize().hex(),n=ix,flags=flags,reportflags=True,txo=dict(lock=out.script_pubkey.raw_serialize().hex(),value=out.amount))
    cli=['container','exec','-i',os.environ.get('BSV_CONTAINER','cg-chronicle-context-v3'),'bitcoin-cli','-regtest','-rpcuser=cgtest','-rpcpassword=local-regtest-only','-stdin','verifyscript']
    answer=json.loads(subprocess.check_output(cli,input=json.dumps([request])+'\nfalse\n10000\n',text=True))[0]
    assert answer.get('flags') == flags, ('unexpected effective flags',answer)
    try:
        tx.validate(parents)
        rust=True; error=None
    except ValueError as e:
        rust=False; error=str(e)
    records.append(dict(request=request,parents=[p.serialize().hex() for p in parents],expected=valid,library=rust,library_error=error,node=answer))
    assert answer['result']==('ok' if valid else 'error'),answer
    if args.mode!='baseline': assert rust==valid,(rust,valid,error)
    return answer

t.check=check
t.UniversalTests.setUpClass()
case=t.UniversalTests()
if args.mode=='baseline':
    ps=[case.fee,case.p]
    for flag in (0x43,0xc3,0x63,0xe3):
        a,sig=case.data(ps,1,[900],flag)
        tx=t.child(ps,[900],{1:case.unlock(ps,1,a,sig)})
        check(tx,ps,1)
        records[-1]['flag']=hex(flag)
        print(hex(flag),records[-1]['library'],records[-1]['node']['result'],flush=True)
else:
    import unittest
    suite=unittest.TestSuite(t.UniversalTests(n) for n in unittest.defaultTestLoader.getTestCaseNames(t.UniversalTests) if n!='test_small_sample')
    result=unittest.TextTestRunner(verbosity=2).run(suite)
json.dump(records,open(args.output,'w'),indent=2)
if args.mode!='baseline' and not result.wasSuccessful(): sys.exit(1)
