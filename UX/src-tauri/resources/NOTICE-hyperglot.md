# Hyperglot — language support data

FontAtlas uses language orthography data (base letters and combining marks per language)
from **Hyperglot**, to tell which languages a font supports.

- Project: Hyperglot — https://github.com/rosettatype/hyperglot
- Authors: Rosetta Type/Research and the Hyperglot community of contributors
  (https://github.com/rosettatype/hyperglot/blob/master/CONTRIBUTORS.txt)
- License: Apache License 2.0 (full text: `LICENSE-hyperglot.txt`)
- Version used: 0.8.2, commit `12d0c093c800a3966e0fd84f60b492cb670696d3`
- Files used: `lib/hyperglot/data/*.yaml` and `lib/hyperglot/extra_data/default.yaml` only.
  The `other/` directory (third-party sources, each with its own license) is NOT used.

## Changes
The YAML data was converted to one compact JSON file (`langdata.json`) by
`tools/langdata/build.py`: only languages with validity `verified` or `preliminary`, only
orthographies with status `primary`, inherited characters resolved, orthographies whose letters
do not match their declared script dropped. No Hyperglot code is included or run.
