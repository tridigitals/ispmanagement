#!/usr/bin/env python3
"""Dump user-facing literals with context for i18n spec authoring."""
import re, sys
fp = sys.argv[1]
s = open(fp).read()
cut = s.rfind('</script>')
tpl = s[cut + 9:]
scr = s[:cut]
seen = []

def add(kind, t, ln):
    t = t.strip()
    if len(t) < 2 or re.match(r'^[\W\d_{}]+$', t): return
    key = (kind, t)
    if key in seen: return
    seen.append(key)
    print(f'{kind:4s} L{ln:4d} | {t[:95]}')

for i, line in enumerate(tpl.splitlines(), start=s.count('\n', 0, cut) + 1):
    for m in re.finditer(r'(placeholder|aria-label|title|label|hint|desc|message|empty|action|detail)="([^"]{2,})"', line):
        add('ATTR', m.group(2), i)
    for m in re.finditer(r">\s*([A-Za-zÀ-ÿ][^<>{}]{2,80}?)\s*<", line):
        t = m.group(1)
        if re.search(r'[a-z].{2} [a-z]|[A-Z][a-z]+ [A-Z][a-z]|\.|!|\?', t) and not re.match(r'^(input|button|span|div|td|th|tr|option|small|strong|code|table|thead|tbody|nav|main|section|header|footer|form|label|a|p|h[1-6])$', t):
            add('TXT', t, i)
for m in re.finditer(r"(toast\.\w+|notice\.\w+)\(\s*['\"`]([^'\"`\n$]{4,120})", scr):
    add('TOAST', m.group(1), scr[:m.start()].count('\n') + 1)
for i, line in enumerate(tpl.splitlines(), start=1):
    for m in re.finditer(r"'([^'\n$]{4,80})'", line):
        t = m.group(1)
        if re.search(r'[a-z] [a-z]|\.|!|\?|[A-Z][a-z]+$', t) and not re.search(r'^[a-z_.-]+$', t) and 'class' not in line.split(t)[0][-14:]:
            add('EXPR', t, s[:cut].count('\n') + i)
for i, line in enumerate(scr.splitlines(), start=1):
    if re.search(r'label:|title:|text:|hint:|desc', line):
        for m in re.finditer(r":\s*'([^'\n]{3,80})'", line):
            t = m.group(1)
            if re.search(r'[a-zA-ZÀ-ÿ] [a-zA-ZÀ-ÿ]', t) and not re.match(r'^[a-z_-]+$', t):
                add('SCRIPT', t, i)
        for m in re.finditer(r'`([^`\n]{4,90})`', line):
            t = m.group(1)
            if '${' in t:
                add('SCRIPT-TL', t, i)
