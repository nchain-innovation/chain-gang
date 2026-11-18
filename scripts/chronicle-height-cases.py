#!/usr/bin/env python3
"""Turn context test exports into requests with node-derived era flags.
Requires the isolated regtest node (activation=200) to have reached height >=200.
"""
import json, os, subprocess, sys
cli=['container','exec',os.environ.get('BSV_CONTAINER','cg-chronicle-context-v3'),
     'bitcoin-cli','-regtest','-rpcuser=cgtest','-rpcpassword=local-regtest-only']
parents={h:subprocess.check_output(cli+['getblockhash',str(h)],text=True).strip() for h in (198,199)}
rows=[]
for row in json.load(open(sys.argv[1])):
    # This group deliberately uses consensus NOP behavior. Node-derived RPC flags
    # include DISCOURAGE_UPGRADABLE_NOPS policy, which is a different comparison.
    if row['name'].startswith('nop-left'):
        continue
    request=row['request']
    flags=request.pop('flags')
    request['prevblockhash']=parents[199 if flags & (1 << 20) else 198]
    request['txo']['height']=200 if flags & (1 << 21) else 100
    rows.append(row)
json.dump(rows,open(sys.argv[2],'w'),indent=2)
