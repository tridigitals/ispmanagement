#!/usr/bin/env python3
"""Build an i18n spec JSON from a compact TSV table.
TSV columns (tab-sep): scope  literal  key-or-ref  en-text
 scope:
   N <nsfile>  -> new key under <nsfile>.json; key = key-or-ref (file-relative); en-text required
   C           -> reuse common.json: key-or-ref = dotted under common (e.g. 'save')
   A           -> reuse admin.json: key-or-ref = dotted under admin
   S           -> reuse other stem: key-or-ref = '<stem>.<dotted>'
   T           -> toast (new under admin): key + en
   P           -> post: old=literal, new=key-or-ref (literal is 'OLD|||NEW')
Output: JSON spec for scripts/i18n_convert.py at $4.
Usage: i18n_spec.py <page-file> <nsfile> <spec-out.json> < <table.tsv>
"""
import sys, json
page, nsfile, out = sys.argv[1], sys.argv[2], sys.argv[3]
new_keys, m, toast, post = {}, {}, {}, []
STEMS = ('admin', 'common', 'dashboard', 'network', 'support', 'components', 'pages', 'payment', 'superadmin', 'profile', 'announcements', 'notifications', 'notifications_page', 'topbar', 'utils', 'install', 'mixradius', 'auth', 'sidebar')

def full_ref(nsfile, key):
    head = key.split('.')[0]
    return key if head in STEMS else f'{nsfile}.{key}'

for ln in sys.stdin:
    ln = ln.rstrip('\n')
    if not ln.strip() or ln.startswith('#'): continue
    parts = ln.split('\t')
    scope = parts[0]
    if scope == 'N':
        _, lit, key, en = parts
        nk = key[len(nsfile) + 1:] if key.startswith(f'{nsfile}.') else key
        new_keys[nk] = {'id': lit, 'en': en}
        m[lit] = full_ref(nsfile, key)
    elif scope == 'A':
        _, lit, key = (parts + [None])[:3]
        m[lit] = key if key.split('.')[0] in STEMS else f'admin.{key}'
    elif scope == 'C':
        _, lit, key = (parts + [None])[:3]
        m[lit] = key if key.split('.')[0] in STEMS else f'common.{key}'
    elif scope == 'S':
        _, lit, ref = (parts + [None])[:3]
        m[lit] = ref
    elif scope == 'NT':
        _, lit, key, en = parts
        new_keys[key] = {'id': lit, 'en': en}
        toast[lit] = f'{nsfile}.{key}'
    elif scope == 'P':
        _, old, new = parts
        post.append({'old': old.replace('\\n', '\n'), 'new': new.replace('\\n', '\n')})
spec = {'file': page, 'nsfile': nsfile, 'new_keys': new_keys, 'map': m, 'toast': toast, 'post': post}
json.dump(spec, open(out, 'w'), ensure_ascii=False, indent=1)
print(f'{out}: {len(m)} map, {len(toast)} toast, {len(new_keys)} new, {len(post)} post')
