#!/usr/bin/env python3
"""从 PubChem API 补充现有药物的化学属性。

读取 data/knowledge.json 中的 drugs，通过 PubChem PUG REST API 查询：
- 分子式 (MolecularFormula)
- 分子量 (MolecularWeight)
- SMILES (CanonicalSMILES)
- InChIKey
- 化合物 CID

运行：python tools/enrich_pubchem.py
"""

import json
import time
import urllib.request
import urllib.error
import os

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA_PATH = os.path.join(BASE_DIR, "data", "knowledge.json")

# PubChem PUG REST API 基础 URL
PUBCHEM_BASE = "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound"

# 需要查询的属性
PROPERTIES = "MolecularFormula,MolecularWeight,CanonicalSMILES,InChIKey,IUPACName"


def query_pubchem(name):
    """通过化合物名称查询 PubChem，返回属性字典或 None。"""
    url = f"{PUBCHEM_BASE}/name/{urllib.parse.quote(name)}/property/{PROPERTIES}/JSON"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "vet-knowledge/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())
        props = data.get("PropertyTable", {}).get("Properties", [])
        if props:
            return props[0]
        return None
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None  # 未找到
        print(f"  [WARN] HTTP {e.code} for {name}")
        return None
    except Exception as e:
        print(f"  [WARN] {name}: {e}")
        return None


def main():
    with open(DATA_PATH, "r", encoding="utf-8") as f:
        data = json.load(f)

    # Windows: 确保 stdout 用 UTF-8
    import sys
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")

    drugs = data.get("drugs", [])
    print(f"共 {len(drugs)} 种药物，开始查询 PubChem...\n")

    found = 0
    not_found = []
    skipped = 0

    for i, drug in enumerate(drugs):
        drug_id = drug["id"]
        name_zh = drug.get("name_zh", "")
        name_en = drug.get("name_en", "")

        # 已有 pubchem_cid 则跳过
        if drug.get("pubchem_cid"):
            skipped += 1
            continue

        # 优先用英文名查询，中文名 PubChem 通常找不到
        query_name = name_en if name_en else name_zh
        if not query_name:
            not_found.append(f"{drug_id}: 无名称")
            continue

        # 清理名称（去掉括号内容、剂型描述）
        clean_name = query_name.split("(")[0].split("（")[0].strip()
        # 取第一段（有些名称用 / 或 , 分隔多种药物）
        clean_name = clean_name.split("/")[0].split(",")[0].strip()

        print(f"[{i+1}/{len(drugs)}] {drug_id}: {clean_name}", end="")

        result = query_pubchem(clean_name)
        if result:
            drug["pubchem_cid"] = result.get("CID")
            drug["molecular_formula"] = result.get("MolecularFormula")
            drug["molecular_weight"] = result.get("MolecularWeight")
            drug["smiles"] = result.get("CanonicalSMILES")
            drug["inchikey"] = result.get("InChIKey")
            drug["iupac_name"] = result.get("IUPACName")
            found += 1
            print(f"  -> CID {result.get('CID')}")
        else:
            not_found.append(f"{drug_id}: {clean_name}")
            print("  -> 未找到")

        # PubChem 频率限制：每秒最多 5 次请求
        time.sleep(0.25)

    # 保存
    with open(DATA_PATH, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"\n{'='*50}")
    print(f"查询完成:")
    print(f"  找到: {found}")
    print(f"  跳过(已有): {skipped}")
    print(f"  未找到: {len(not_found)}")
    if not_found:
        print(f"\n未找到的药物:")
        for item in not_found:
            print(f"  - {item}")


if __name__ == "__main__":
    main()
