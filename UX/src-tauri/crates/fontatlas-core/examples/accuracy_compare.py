"""Compare two collect.py outputs from the independent audit; no font files are modified.

python3 examples/accuracy_compare.py --before before/rows.json --after after/rows.json \
  --audit-dir /path/to/fa-audit --out summary.json
"""
import argparse
import collections
import importlib.util
import json
import math
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--before', type=Path, required=True)
p.add_argument('--after', type=Path, required=True)
p.add_argument('--audit-dir', type=Path, required=True)
p.add_argument('--out', type=Path, required=True)
a = p.parse_args()
spec = importlib.util.spec_from_file_location('audit_labels', a.audit_dir / 'lab.py')
labels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(labels)
before, after = (json.loads(path.read_text()) for path in (a.before, a.after))
key = lambda row: (row['file'], row['index'])
old, new = ({key(r): r for r in rows} for rows in (before, after))
assert old.keys() == new.keys(), 'before/after must contain the same faces'
valid = [k for k, r in old.items() if 'err' not in r and 'err' not in new[k]]
latin = [k for k in valid if old[k]['has_az']]
value = lambda r, axis: r['an'][axis]['value']
upright = lambda r: abs(r['ia']) < .5 and not re.search('Italic|Oblique', r['name'] + r['file']) and abs(value(r, 'slant_deg') or 0) < 3
sans = [k for k in latin if labels.label(old[k])[0] == 'sans' and upright(old[k])]
condensed = [k for k in latin if re.search('Condensed|Narrow|Bebas|Oswald|Impact', old[k]['name'])]

def count_where(keys, fn):
    return {phase: sum(bool(fn(rows[k])) for k in keys) for phase, rows in [('before', old), ('after', new)]}

def quality(r, axis):
    q = r['an'][axis]['quality']
    return q['kind'] + (':' + q['reason'] if q.get('reason') else '')

result = {'faces': {'raw': len(old), 'valid': len(valid), 'latin': len(latin), 'sans_upright': len(sans), 'condensed': len(condensed)}}
result['quality_latin'] = {axis: {
    'before': dict(sorted(collections.Counter(quality(old[k], axis) for k in latin).items())),
    'after': dict(sorted(collections.Counter(quality(new[k], axis) for k in latin).items())),
    'changed': sum(old[k]['an'][axis]['quality'] != new[k]['an'][axis]['quality'] for k in latin),
} for axis in old[latin[0]]['an'] if axis != 'location'}
result['sans_stress_estimated'] = count_where(sans, lambda r: quality(r, 'stress_deg') == 'estimated')
result['stress_outside_45_latin'] = count_where(latin, lambda r: abs(value(r, 'stress_deg') or 0) > 45)
result['condensed_stress_90'] = count_where(condensed, lambda r: abs(value(r, 'stress_deg') or 0) == 90)
result['negative_zero_stress_latin'] = count_where(latin, lambda r: value(r, 'stress_deg') == 0 and math.copysign(1, value(r, 'stress_deg')) < 0)
result['changed_values_all_valid'] = {axis: sum(value(old[k], axis) != value(new[k], axis) for k in valid) for axis in old[latin[0]]['an'] if axis != 'location'}
result['remaining_sans_estimated'] = [old[k]['name'] for k in sans if quality(new[k], 'stress_deg') == 'estimated']
a.out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(result, ensure_ascii=False, indent=2))
