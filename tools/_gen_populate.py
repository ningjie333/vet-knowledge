#!/usr/bin/env python3
"""Generate the populate_drug_fields.py script with all drug data."""

import os

DRUGS_DIR = os.path.dirname(os.path.abspath(__file__))

drug_entries = []
for i in range(1, 61):
    drug_entries.append(f'    "drug_{i:03d}": {{')
    drug_entries.append(f'        "mechanism_of_action": "",')
    drug_entries.append(f'        "pk_pd": "",')
    drug_entries.append(f'        "adverse_mechanism": ""')
    drug_entries.append(f'    }},')

data_str = "\n".join(drug_entries)

script = f'''#!/usr/bin/env python3
"""
Populate mechanism_of_action, pk_pd, and adverse_mechanism fields
for all drug MD files in data/drugs/.
Skips files that already have these fields filled.
"""

import frontmatter
import os
import glob
import sys

DRUGS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "drugs")

DRUG_DATA = {{
{data_str}
}}

NEW_FIELDS = ["mechanism_of_action", "pk_pd", "adverse_mechanism"]

def process_drug(filepath):
    """Process a single drug file. Returns (drug_id, fields_added, skipped)."""
    filename = os.path.basename(filepath)
    drug_id = os.path.splitext(filename)[0]

    if drug_id not in DRUG_DATA:
        print(f"  WARNING: No data for {drug_id}, skipping")
        return (drug_id, 0, 0)

    data = DRUG_DATA[drug_id]

    # Check if any data is empty
    for field in NEW_FIELDS:
        if not data.get(field, "").strip():
            print(f"  WARNING: {drug_id} has empty {field}")
            return (drug_id, 0, 0)

    # Read existing file
    with open(filepath, "r", encoding="utf-8") as f:
        post = frontmatter.load(f)

    # Check which fields already have values
    added = 0
    skipped = 0
    for field in NEW_FIELDS:
        existing = post.metadata.get(field, "")
        if existing and str(existing).strip():
            skipped += 1
        else:
            post.metadata[field] = data[field]
            added += 1

    if added > 0:
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(frontmatter.dumps(post))

    return (drug_id, added, skipped)

def main():
    files = sorted(glob.glob(os.path.join(DRUGS_DIR, "drug_*.md")))
    total_files = len(files)
    total_added = 0
    total_skipped = 0
    processed = 0

    print(f"Processing {{total_files}} drug files in {{DRUGS_DIR}}")
    print("-" * 60)

    for filepath in files:
        filename = os.path.basename(filepath)
        drug_id, added, skipped = process_drug(filepath)
        processed += 1
        total_added += added
        total_skipped += skipped

        status = []
        if added > 0:
            status.append(f"added {{added}}")
        if skipped > 0:
            status.append(f"skipped {{skipped}} (already set)")
        if not status:
            status_str = "no data or empty"
        else:
            status_str = ", ".join(status)

        print(f"  {{processed}}/{{total_files}} {{filename}}: {{status_str}}")

    print("-" * 60)
    print(f"Done. Processed {{processed}} files. Added {{total_added}} fields. Skipped {{total_skipped}} already-set fields.")

if __name__ == "__main__":
    main()
'''

output_path = os.path.join(DRUGS_DIR, "populate_drug_fields.py")
with open(output_path, "w", encoding="utf-8") as f:
    f.write(script)

print(f"Generated {output_path}")
print(f"Script contains {len(DRUG_DATA)} drug entries (with empty fields)")
print("You still need to fill in the actual field values in DRUG_DATA")
