#!/usr/bin/env python3
"""Compare exported Rust cases with BSV v1.2.3 verifyscript in an isolated container.
Run cargo test with CHRONICLE_DIFF_OUTPUT=/absolute/cases.json, then:
python3 scripts/chronicle-node-diff.py cases.json results.json
BSV_CONTAINER selects the isolated node (default cg-chronicle-node).
The node's validation-duration policy must be long enough for large scripts.
Timeout, skipped, missing results and RPC errors are failures, never rejection.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

def flags_match(row, reply):
    request = row['request']
    if 'flags' in request:
        return reply.get('flags') == request['flags']
    if 'expected_era' not in row:
        if 'prevblockhash' in request:
            raise ValueError('Missing expected_era. Regenerate height cases with chronicle-height-cases.py')
        return True
    flags = reply.get('flags')
    return type(flags) is int and row['expected_era'] == {
        'chronicle': bool(flags & (1 << 20)),
        'utxo_after_chronicle': bool(flags & (1 << 21)),
    }


cases = json.loads(Path(sys.argv[1]).read_text())
if not isinstance(cases, list) or not cases:
    raise ValueError('Expected a non-empty list of replay cases')
node = os.environ.get('BSV_CONTAINER', 'cg-chronicle-node')
runtime = os.environ.get('BSV_CONTAINER_RUNTIME', 'container')
if runtime not in ('container', 'docker'):
    raise ValueError('BSV_CONTAINER_RUNTIME must be container or docker')
cli = [runtime, 'exec', '-i', node, 'bitcoin-cli', '-regtest',
       '-rpcuser=cgtest', '-rpcpassword=local-regtest-only', '-stdin']
version = subprocess.check_output(
    [runtime, 'exec', node, 'bitcoind', '--version'], text=True).splitlines()[0]
if not re.search(r'\bv?1\.2\.3(?:\.0)?(?=\s|$|[-+])', version):
    raise RuntimeError(f'Expected BSV Node v1.2.3, got {version!r}')
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
        description_matches = (row.get('expected_description') is None or
                               reply.get('description') == row['expected_description'])
        matches = (flags_match(row, reply) and description_matches and reply['result'] in ('ok', 'error') and
                   (reply['result'] == 'ok') == row['library'] == row['expected'])
        results.append({**row, 'node': reply, 'matches': matches})
        if not matches:
            print(row['name'], 'MISMATCH', reply, flush=True)
    print(f'{len(results)}/{len(cases)} checked', flush=True)
Path(sys.argv[2]).write_text(json.dumps({
    'node_version': version, 'runtime': runtime, 'cases': results,
}, indent=2))
sys.exit(0 if all(r['matches'] for r in results) else 1)
