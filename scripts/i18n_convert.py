#!/usr/bin/env python3
"""Declarative i18n converter for a Svelte page.
SPEC = {
  'file': path,
  'new_keys': {'a.b': {'id':..., 'en':...}},     # keys to add under ADMIN root (admin.json) unless key starts with 'common.'/'pages.' etc.
  'nsfile': 'admin',                              # namespace file to write new_keys into (default admin)
  'map': {  # ordered list of (raw_literal, '$t_key') — literal matched as quoted string in template OR as plain text between tags OR attr value
  },
  'toast': { raw_message: key },  # toast.*('msg') -> toast.*($t('key'))
}
Modes per map entry key string:
  'attr:NAME|"value"' handled automatically: value occurrence inside quotes for that attr.
Simplest: we search `="value"` and `'value'` and `"value"` and `>value<` — any occurrence of the exact literal (respecting delimiters), replace with the $t form appropriate for the context:
  - inside ="..."  -> ={ $t('key') }
  - inside '...' or "..." as JS string in script -> $t('key')  (only if template expression context — handled by { } wrapper)
We'll only do template-level: attribute values (="v" -> ={ $t } ) , text nodes ( >v< -> {$t} ), and JS-in-template expressions inside { } where literal appears quoted -> replace literal with $t('key').
"""
import json, re, sys
from pathlib import Path

def load_ns(lang):
    return json.load(open(f'src/lib/i18n/namespaces/{lang}/admin.json'))

def deep_set(d, dotted, val):
    parts = dotted.split('.')
    for p in parts[:-1]:
        d = d.setdefault(p, {})
    d[parts[-1]] = val

def merge_new_keys(nsfile, new_keys):
    for lang in ('id', 'en'):
        fp = f'src/lib/i18n/namespaces/{lang}/{nsfile}.json'
        d = json.load(open(fp))
        for k, v in new_keys.items():
            if k in v and isinstance(v[k], dict) and False:
                pass
            deep_set(d, k, v[lang])
        json.dump(d, open(fp, 'w'), ensure_ascii=False, indent=2)
        open(fp, 'a').write('\n')

def ensure_ts_import(s):
    if re.search(r"import \{ t \} from 'svelte-i18n'", s):
        return s
    anchor = None
    for mm in re.finditer(r"^[ \t]*import [^\n]*?;\s*$", s, re.M):
        anchor = mm
    if anchor:
        return s[:anchor.end()] + "\n  import { t } from 'svelte-i18n';" + s[anchor.end():]
    raise RuntimeError('no imports found')

def esc_re(txt):
    return re.escape(txt)

def convert_page(spec):
    fp = spec['file']
    s = open(fp).read()
    nsfile = spec.get('nsfile', 'admin')
    report = []
    if spec.get('new_keys'):
        merge_new_keys(nsfile, spec['new_keys'])
    uses_t = bool(spec.get('map') or spec.get('toast') or spec.get('rawexpr'))
    if uses_t:
        s = ensure_ts_import(s)
    for raw, key in spec.get('map', {}).items():
        before = s
        # 1) HTML attribute values:  attr="raw"  ->  attr={ $t('key') }
        s = re.sub(r'(\b[a-zA-Z-]+=)"' + esc_re(raw) + '"', lambda m: m.group(1) + "{ $t('" + key + "') }", s)
        # 2) text node: >raw<
        s = re.sub(r'>' + esc_re(raw) + r'<', ">{ $t('" + key + "') }<", s)
        # 3) single/double-quoted literal in template expressions: 'raw' or "raw"
        s = re.sub(r"'" + esc_re(raw) + r"'", "$t('" + key + "')", s)
        s = re.sub(r'"' + esc_re(raw) + r'"', "$t('" + key + "')", s)
        # avoid double-wrap: replace ">{ $t(" leftovers fine; but attr now ={ $t(...) } attr={ $t() } OK
        report.append(('map', raw, key, before != s))
    for raw, key in spec.get('toast', {}).items():
        before = s
        # toast.error(`...${x}...`) / toast.error('...')  — handle template literal with ${} by keeping interpolation
        # plain single/double quote
        s = re.sub(r"(toast\.\w+\()\s*'" + esc_re(raw) + r"'\s*\)", lambda m: m.group(1) + "$t('" + key + "'))", s)
        s = re.sub(r'(toast\.\w+\()\s*"' + esc_re(raw) + r'"\s*\)', lambda m: m.group(1) + "$t('" + key + "'))", s)
        # backtick no-interpolation
        s = re.sub(r"(toast\.\w+\()\s*`" + esc_re(raw) + r"`\s*\)", lambda m: m.group(1) + "$t('" + key + "'))", s)
        # backtick with ${...} interpolation: handled by explicit post-pass strings (engine pass removed: too blind)
        # string + concat: 'prefix' + x handled by map plain mode
        # normalize $t('k', {a: x}) -> $t('k', { values: {a: x} }) for TS MessageObject
        s = re.sub(r"\$t\('([^']+)',\s*\{(?!\s*(?:values|default)\b)([^{}]+)\}", r"$t('\1', { values: {\2}", s)
        report.append(('toast', raw, key, before != s))
    for raw, key in spec.get('rawexpr', {}).items():
        s = re.sub(esc_re(raw), "$t('" + key + "')", s)
        report.append(('rawexpr', raw, key, True))
    for entry in spec.get('post', []):
        a, b = entry['old'], entry['new']
        if a in s:
            s = s.replace(a, b)
            report.append(('post', a[:60], 'post', True))
        else:
            report.append(('post', a[:60], 'post', False))
    open(fp, 'w').write(s)
    ok = sum(1 for r in report if r[3])
    print(f'{fp}: applied {ok}/{len(report)}')
    for kind, raw, key, hit in report:
        if not hit:
            print(f'  MISS [{kind}] {raw[:50]!r} -> {key}')
