#!/usr/bin/env python3
"""Build the drug_fields_data.py module with all drug field values."""

import os
import json

TOOLS_DIR = os.path.dirname(os.path.abspath(__file__))

# Read field values from a JSON file
JSON_PATH = os.path.join(TOOLS_DIR, "drug_fields.json")

with open(JSON_PATH, "r", encoding="utf-8") as f:
    DRUG_DATA = json.load(f)

# Write the Python data module
output = '''#!/usr/bin/env python3
"""Auto-generated drug field data for populate_drug_fields.py"""

DRUG_DATA = {
'''

for drug_id in sorted(DRUG_DATA.keys()):
    data = DRUG_DATA[drug_id]
    output += f'    "{drug_id}": {{\n'
    output += f'        "mechanism_of_action": {json.dumps(data["mechanism_of_action"], ensure_ascii=False)},\n'
    output += f'        "pk_pd": {json.dumps(data["pk_pd"], ensure_ascii=False)},\n'
    output += f'        "adverse_mechanism": {json.dumps(data["adverse_mechanism"], ensure_ascii=False)},\n'
    output += f'    }},\n'

output += "}\n"

DATA_PY_PATH = os.path.join(TOOLS_DIR, "drug_fields_data.py")
with open(DATA_PY_PATH, "w", encoding="utf-8") as f:
    f.write(output)

print(f"Generated {DATA_PY_PATH} with {len(DRUG_DATA)} drug entries")
