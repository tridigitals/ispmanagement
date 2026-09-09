#!/bin/bash
# Apply i18n specs serially + validate each. Usage: ./scripts/i18n_batch.sh spec1.json spec2.json ...
set -e
cd /home/xtrabit/ISPMANAGEMENT
for spec in "$@"; do
  echo "── APPLY $spec"
  python3 scripts/i18n_apply.py "$spec"
done
echo "── CHECK KEYS"
mapfile -t FILES < <(python3 -c "
import json,sys
for f in sys.argv[1:]:
    print(json.load(open(f))['file'])
" "$@")
python3 scripts/i18n_check.py "${FILES[@]}"
echo "── SVELTE-CHECK"
npx svelte-check --tsconfig ./tsconfig.json --output machine 2>&1 | grep -E 'COMPLETED|ERROR' | tail -5
