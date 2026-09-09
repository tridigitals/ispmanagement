#!/usr/bin/env python3
"""Verify all $t('key') refs in given files resolve in id+en namespaces.
Key roots = namespace file stem (admin.network.x -> namespaces/en/admin.json.network.x)."""
import json, re, sys, os
os.chdir(os.path.join(os.path.dirname(__file__), '..'))

NSTOX = {}
for lang in ('id', 'en'):
    for fn in os.listdir(f'src/lib/i18n/namespaces/{lang}'):
        if fn.endswith('.json'):
            NSTOX[(lang, fn[:-5])] = json.load(open(f'src/lib/i18n/namespaces/{lang}/{fn}'))

def resolve(root_key, dotted):
    cur = root_key
    for p in dotted.split('.'):
        if isinstance(cur, dict) and p in cur:
            cur = cur[p]
        else:
            return None
    return cur if isinstance(cur, str) else None

def check(fp):
    src = open(fp).read()
    keys = set(re.findall(r"\$t\(\s*'([^']+)'", src))
    miss = []
    for k in sorted(keys):
        parts = k.split('.')
        stem = parts[0]
        rest = '.'.join(parts[1:])
        for lang in ('id', 'en'):
            root = NSTOX.get((lang, stem))
            if root is None or not rest:
                miss.append((k, lang, 'no-ns' if root is None else 'empty-path'))
                continue
            v = resolve(root, rest)
            if v is None:
                miss.append((k, lang, 'missing'))
    return keys, miss

bad = 0
for fp in sys.argv[1:]:
    keys, miss = check(fp)
    if miss:
        bad += len(miss)
        for k, lang, why in miss:
            print(f'MISS {k} [{lang}] {why}  in {fp}')
    else:
        print(f'OK {fp}  ({len(keys)} keys)')
sys.exit(1 if bad else 0)
