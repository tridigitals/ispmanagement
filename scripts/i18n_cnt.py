#!/usr/bin/env python3
"""Count exact substring occurrences per line from a list file."""
import sys
from pathlib import Path
fp = sys.argv[1]
s = open(fp).read()
for raw in Path(sys.argv[2]).read_text().splitlines():
    if not raw.strip(): continue
    print(f'{s.count(raw)} :: {raw[:70]}')
