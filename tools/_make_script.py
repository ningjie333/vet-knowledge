#!/usr/bin/env python3
"""Generate populate_drug_fields.py with all 60 drugs' data."""
import os, json, frontmatter

BASE = os.path.dirname(os.path.abspath(__file__))
DRUGS_DIR = os.path.join(BASE, "..", "data", "drugs")

# First, scan all drug files to get their metadata
files = sorted(os.listdir(DRUGS_DIR))
drug_info = {}
for fn in files:
    if not fn.endswith(".md"):
        continue
    fp = os.path.join(DRUGS_DIR, fn)
    with open(fp, "r", encoding="utf-8") as f:
        post = frontmatter.load(f)
    drug_id = os.path.splitext(fn)[0]
    drug_info[drug_id] = {
        "drug_class": post.metadata.get("drug_class", ""),
        "name_zh": post.metadata.get("name_zh", ""),
        "name_en": post.metadata.get("name_en", ""),
    }

# Now build the complete script
lines = []
lines.append('#!/usr/bin/env python3')
lines.append('"""Populate mechanism_of_action, pk_pd, and adverse_mechanism fields for drug MD files."""')
lines.append('import frontmatter, os, glob')
lines.append('')
lines.append('DRUGS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "drugs")')
lines.append('NEW_FIELDS = ["mechanism_of_action", "pk_pd", "adverse_mechanism"]')
lines.append('')
lines.append('DRUG_DATA = {')

# We'll fill in the data below by reading from a JSON file
# For now, just create the scaffold
lines.append('}')
lines.append('')
lines.append('def main():')
lines.append('    files = sorted(glob.glob(os.path.join(DRUGS_DIR, "drug_*.md")))')
lines.append('    for fp in files:')
lines.append('        fn = os.path.basename(fp)')
lines.append('        did = os.path.splitext(fn)[0]')
lines.append('        if did not in DRUG_DATA:')
lines.append('            print(f"  SKIP {did}: no data"); continue')
lines.append('        d = DRUG_DATA[did]')
lines.append('        for fld in NEW_FIELDS:')
lines.append('            if not d.get(fld, "").strip():')
lines.append('                print(f"  SKIP {did}: empty {fld}"); break')
lines.append('        else:')
lines.append('            with open(fp, "r", encoding="utf-8") as f:')
lines.append('                post = frontmatter.load(f)')
lines.append('            added = 0')
lines.append('            for fld in NEW_FIELDS:')
lines.append('                if not post.metadata.get(fld, "").strip():')
lines.append('                    post.metadata[fld] = d[fld]; added += 1')
lines.append('            if added:')
lines.append('                with open(fp, "w", encoding="utf-8") as f:')
lines.append('                    f.write(frontmatter.dumps(post))')
lines.append('            print(f"  {fn}: added {added}")')
lines.append('')
lines.append('if __name__ == "__main__": main()')

scaffold = "\n".join(lines)

# Write scaffold
out = os.path.join(BASE, "populate_drug_fields.py")
with open(out, "w", encoding="utf-8") as f:
    f.write(scaffold)

print(f"Wrote scaffold to {out}")
print(f"Found {len(drug_info)} drug files")
for did in sorted(drug_info.keys()):
    info = drug_info[did]
    print(f"  {did}: {info['name_zh']} ({info['drug_class']})")
