#!/usr/bin/env python3
"""
Populate mechanism_of_action, pk_pd, and adverse_mechanism fields
for all drug MD files in data/drugs/.
Skips files that already have these fields filled.
Reads field data from drug_fields_data.py (generated separately).
"""

import frontmatter
import os
import glob
import sys

DRUGS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "drugs")

# Import the data module
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from drug_fields_data import DRUG_DATA

NEW_FIELDS = ["mechanism_of_action", "pk_pd", "adverse_mechanism"]

def process_drug(filepath):
    """Process a single drug file. Returns (drug_id, fields_added, skipped)."""
    filename = os.path.basename(filepath)
    drug_id = os.path.splitext(filename)[0]

    if drug_id not in DRUG_DATA:
        print(f"  WARNING: No data for {drug_id}, skipping")
        return drug_id, 0, 0

    data = DRUG_DATA[drug_id]

    # Check if any data is empty
    for field in NEW_FIELDS:
        if not data.get(field, "").strip():
            print(f"  WARNING: {drug_id} has empty {field}")
            return drug_id, 0, 0

    # Read existing file
    with open(filepath, "r", encoding="utf-8") as f:
        post = frontmatter.load(f)

    # Check which fields already have values
    added = 0
    skipped_count = 0
    for field in NEW_FIELDS:
        existing = post.metadata.get(field, "")
        if existing and str(existing).strip():
            skipped_count += 1
        else:
            post.metadata[field] = data[field]
            added += 1

    if added > 0:
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(frontmatter.dumps(post))

    return drug_id, added, skipped_count

def main():
    files = sorted(glob.glob(os.path.join(DRUGS_DIR, "drug_*.md")))
    total_files = len(files)
    total_added = 0
    total_skipped = 0
    processed = 0

    print(f"Processing {total_files} drug files in {DRUGS_DIR}")
    print("-" * 60)

    for filepath in files:
        filename = os.path.basename(filepath)
        drug_id, added, skipped_count = process_drug(filepath)
        processed += 1
        total_added += added
        total_skipped += skipped_count

        status_parts = []
        if added > 0:
            status_parts.append(f"added {added}")
        if skipped_count > 0:
            status_parts.append(f"skipped {skipped_count}")
        if not status_parts:
            status_str = "no data"
        else:
            status_str = ", ".join(status_parts)

        print(f"  {processed}/{total_files} {filename}: {status_str}")

    print("-" * 60)
    print(f"Done. {processed} files processed. {total_added} fields added. {total_skipped} already set.")

if __name__ == "__main__":
    main()
