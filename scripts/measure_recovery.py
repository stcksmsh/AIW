#!/usr/bin/env python3
"""Measure the bounded recovery contract using a synthetic terminal history."""
import json
import pathlib
import subprocess
import sys
import tempfile
import time

binary = str(pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/aiw').resolve())
with tempfile.TemporaryDirectory(prefix='aiw-recovery-') as directory:
    def aiw(*args):
        return subprocess.check_output([binary, '-C', directory, *args])

    aiw('init', '--name', 'Recovery benchmark')
    fixture = json.loads(pathlib.Path('examples/minimal-state.json').read_text())
    path = pathlib.Path(directory) / '.ai/state.json'
    path.write_text(json.dumps(fixture))
    before = aiw('load')
    historical = dict(fixture['tasks']['feature'], status='cancelled', title='Historical task')
    for n in range(1500):
        fixture['tasks'][f'history-{n:04}'] = historical
    path.write_text(json.dumps(fixture))
    start = time.monotonic()
    after = aiw('load')
    elapsed = time.monotonic() - start
    constrained = aiw('load', '--budget', '2048')
    assert before == after, 'terminal history leaked into recovery'
    assert len(after) <= 8192 and len(constrained) <= 2048
    print(json.dumps({'history_tasks': 1500, 'before_bytes': len(before),
                      'after_bytes': len(after), 'budget_2048_bytes': len(constrained),
                      'recovery_ms': round(elapsed * 1000, 2)}, indent=2))
