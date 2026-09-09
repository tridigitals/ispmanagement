#!/usr/bin/env python3
"""Reverse index: value text -> dotted keys, for namespace reuse."""
import json, glob, re, sys
from pathlib import Path

idx = {}
def walk(d, prefix, file):
    for k, v in d.items():
        p = f'{prefix}.{k}' if prefix else k
        if isinstance(v, dict):
            walk(v, p, file)
        elif isinstance(v, str) and len(v) >= 3:
            norm = re.sub(r'\{[^}]+\}', ' {} ', v).lower()
            norm = re.sub(r'\s+', ' ', norm).strip()
            idx.setdefault(norm, []).append((file, p, v))

for f in sorted(glob.glob('src/lib/i18n/namespaces/id/*.json')):
    root = Path(f).stem
    walk(json.load(open(f)), '', root)

if __name__ == '__main__':
    for q in sys.argv[1:]:
        norm = re.sub(r'\{[^}]+\}', ' {} ', q).lower()
        norm = re.sub(r'\s+', ' ', norm).strip()
        hits = idx.get(norm, [])
        if not hits:
            hits = [(f2, k, v) for v2, lst in idx.items() for (f2, k, v) in lst if norm and (norm in v2 or v2 in norm) and len(norm) > 10]
        print(f'Q: {q!r}')
        for f2, k, v in hits[:4]:
            print(f'   [{f2}] {k}  = {v[:60]!r}')
        if not hits: print('   (none)')
