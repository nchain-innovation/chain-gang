"""Exercise the replay tools using the requests and replies recorded from Node."""
import contextlib
import io
import json
from pathlib import Path
import runpy
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
SCRIPTS = ROOT / 'scripts'
RECEIPTS = ROOT / 'tests' / 'receipts'


class ChronicleReplayTest(unittest.TestCase):
    def test_height_derived_flags(self):
        explicit = json.loads((RECEIPTS / 'chronicle-context-node.json').read_text())['cases']
        recorded = json.loads((RECEIPTS / 'chronicle-height-derived-node.json').read_text())['cases']
        parents = {r['name']: r['request']['prevblockhash'] for r in recorded}
        block_hashes = [parents['2mul-activefalse-newfalse-v1'], parents['2mul-activetrue-newfalse-v1']]
        with tempfile.TemporaryDirectory() as tmp:
            cases, derived, result = (Path(tmp) / f for f in ('input', 'derived', 'result'))
            cases.write_text(json.dumps(explicit))
            with patch.object(sys, 'argv', ['', str(cases), str(derived)]), patch(
                'subprocess.check_output', side_effect=block_hashes
            ):
                runpy.run_path(str(SCRIPTS / 'chronicle-height-cases.py'))
            rows = json.loads(derived.read_text())
            self.assertEqual(len(rows), 39)
            self.assertEqual([r['name'] for r in rows], [r['name'] for r in explicit])
            self.assertEqual(sum(r['name'].startswith('nop-left') for r in rows), 9)
            self.assertTrue(all('flags' not in r['request'] for r in rows))
            self.assertEqual([r['name'] for r in rows], [r['name'] for r in recorded])
            self.assertEqual([r['request'] for r in rows], [r['request'] for r in recorded])

            def replay(replies):
                with patch.object(sys, 'argv', ['', str(derived), str(result)]), patch(
                    'subprocess.check_output',
                    side_effect=['Bitcoin SV version v1.2.3'] + [
                        json.dumps(replies[i:i + 32]) for i in range(0, len(replies), 32)
                    ],
                ), contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as exit_:
                    runpy.run_path(str(SCRIPTS / 'chronicle-node-diff.py'))
                return exit_.exception.code, [r['matches'] for r in json.loads(result.read_text())['cases']]

            replies = [r['node'] for r in recorded]
            self.assertEqual(replay(replies), (0, [True] * len(rows)))
            for bit in (20, 21):
                with self.subTest(bit=bit):
                    changed = [dict(r, flags=r['flags'] ^ (1 << bit)) for r in replies]
                    self.assertEqual(replay(changed), (1, [False] * len(rows)))
            missing = [{k: v for k, v in r.items() if k != 'flags'} for r in replies]
            self.assertEqual(replay(missing), (1, [False] * len(rows)))
            other = [dict(r, flags=r['flags'] ^ 1) for r in replies]
            self.assertEqual(replay(other), (0, [True] * len(rows)))
