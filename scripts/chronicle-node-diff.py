#!/usr/bin/env python3
"""Compare exported Rust cases with BSV v1.2.3 verifyscript in Apple Container.
Run cargo test with CHRONICLE_DIFF_OUTPUT=/absolute/cases.json, then:
python3 scripts/chronicle-node-diff.py cases.json results.json
BSV_CONTAINER selects the isolated node (default cg-chronicle-node).
The node's validation-duration policy must be long enough for large scripts.
Timeout, skipped, missing results and RPC errors are failures, never rejection.
"""
import json
import os
import subprocess
import sys

cases = json.load(open(sys.argv[1]))
node = os.environ.get('BSV_CONTAINER', 'cg-chronicle-node')
cli = ['container', 'exec', '-i', node, 'bitcoin-cli', '-regtest',
       '-rpcuser=cgtest', '-rpcpassword=local-regtest-only', '-stdin']
version = subprocess.check_output(
    ['container', 'exec', node, 'bitcoind', '--version'], text=True).splitlines()[0]
results = []
for start in range(0, len(cases), 32):
    batch = cases[start:start + 32]
    replies = json.loads(subprocess.check_output(
        cli + ['verifyscript'],
        input=json.dumps([row['request'] for row in batch]) + '\nfalse\n10000\n',
        text=True, timeout=120))
    if len(replies) != len(batch):
        raise RuntimeError('Node returned an incomplete batch')
    for row, reply in zip(batch, replies):
        flags_match = ('flags' not in row['request'] or
                       reply.get('flags') == row['request']['flags'])
        matches = (flags_match and reply['result'] in ('ok', 'error') and
                   (reply['result'] == 'ok') == row['library'] == row['expected'])
        results.append({**row, 'node': reply, 'matches': matches})
        if not matches:
            print(row['name'], 'MISMATCH', reply, flush=True)
    print(f'{len(results)}/{len(cases)} checked', flush=True)
json.dump({'node_version': version,
           'image': 'docker.io/bitcoinsv/bitcoin-sv@sha256:d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33',
           'runtime': 'Apple Container', 'cases': results}, open(sys.argv[2], 'w'), indent=2)
sys.exit(0 if all(r['matches'] for r in results) else 1)
