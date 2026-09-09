#!/usr/bin/env python3
"""Apply one or more i18n spec JSON files via scripts/i18n_convert.py engine."""
import runpy, sys, json, os
os.chdir(os.path.join(os.path.dirname(__file__), '..'))
mod = runpy.run_path('scripts/i18n_convert.py')
convert_page = mod['convert_page']
for f in sys.argv[1:]:
    spec = json.load(open(f))
    convert_page(spec)
