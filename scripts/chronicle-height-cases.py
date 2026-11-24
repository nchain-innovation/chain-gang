#!/usr/bin/env python3
"""Turn context test exports into requests with node-derived era flags.
Requires the isolated regtest node (activation=200) to have reached height >=200.
"""
import json, os, subprocess, sys
from pathlib import Path
cli=['container','exec',os.environ.get('BSV_CONTAINER','cg-chronicle-context-v3'),
     'bitcoin-cli','-regtest','-rpcuser=cgtest','-rpcpassword=local-regtest-only']
parents={h:subprocess.check_output(cli+['getblockhash',str(h)],text=True).strip() for h in (198,199)}
rows=[]
for row in json.loads(Path(sys.argv[1]).read_text()):
    # With prevblockhash, verifyscript derives block flags, so old-output NOP
    # cases remain applicable. Its policy resource limits do not add NOP policy.
    request=row['request']
    flags=request.pop('flags')
    after_activation = bool(flags & (1 << 20))
    new_output = bool(flags & (1 << 21))
    request['prevblockhash'] = parents[199 if after_activation else 198]
    request['txo']['height'] = 200 if new_output else 100
    row['expected_era'] = {'chronicle': after_activation, 'utxo_after_chronicle': new_output}
    rows.append(row)
Path(sys.argv[2]).write_text(json.dumps(rows, indent=2))
