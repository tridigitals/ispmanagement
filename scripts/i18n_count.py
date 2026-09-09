#!/usr/bin/env python3
"""Count remaining raw literals per v2 page."""
import re, sys
from pathlib import Path
ROOT = Path('src/routes')
pages = sorted(ROOT.rglob('+page.svelte'))
tot = 0
rows = []
for p in pages:
    s = p.read_text()
    n_t = len(re.findall(r'\$t\(', s))
    # crude raw count: ATTR + TXT + TOAST patterns
    cut = s.rfind('</script>')
    tpl, scr = s[cut+9:], s[:cut]
    hits = set()
    for m in re.finditer(r'(placeholder|aria-label|title|label|hint|desc|message)="([^"]{4,})"', tpl):
        v = m.group(2)
        if re.search(r'[A-Za-zÀ-ÿ]{3}', v) and '{' not in v and not re.match(r'^[\w-]+ [\w-]+$', v) and 'class' not in v.lower():
            hits.add(v)
    for m in re.finditer(r">\s*([A-Za-zÀ-ÿ][^<>{}\n]{4,80}?)\s*<", tpl):
        t = m.group(1).strip()
        if re.search(r'[a-z] [a-z]|\.|\?|!', t) and not re.match(r'^(input|button|span|div|td|th|tr|option|small|strong|code|table|thead|tbody|nav|main|section|header|footer|form|label|a|p|h[1-6])$', t, re.I):
            hits.add(t)
    for m in re.finditer(r"(toast\.\w+)\(\s*['\"`]([^'\"`\n$]{6,120})", scr):
        hits.add(m.group(2))
    rows.append((len(hits), n_t, str(p)))
    tot += len(hits)
rows.sort(key=lambda r: -r[0])
print(f'pages-with-raw={sum(1 for r in rows if r[0])}  total-raw={tot}')
for n, nt, path in rows:
    if n: print(f'{n:4d} raw | {nt:3d} $t | {path}')
