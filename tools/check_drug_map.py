#!/usr/bin/env python3
"""药物映射表一致性校验（data/game_drug_to_vk_id.json ↔ data/knowledge.json）。

断言：
  (a) 映射表里每个 drug id 都能在 knowledge.json 的 drugs 数组中找到；
  (b) 映射表每个键的规范英文名 = 所指药物的 name_en（允许括号别名、"/"并列成分、
      首字母缩略词，以及 SPELLING_VARIANTS 里登记的 USAN/INN 同药异拼）；
  (c) drugs 数组内不存在规范英文名重复的两条记录（重名会导致同一个药有两个 id）；
  (d) drug_interactions 引用的 drug_a / drug_b 都存在。

任一断言不满足 → 打印明细并以退出码 1 结束。
用法：python tools/check_drug_map.py
"""

import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")

ROOT = Path(__file__).resolve().parent.parent
KNOWLEDGE_PATH = ROOT / "data" / "knowledge.json"
MAP_PATH = ROOT / "data" / "game_drug_to_vk_id.json"

# 规范化时忽略的连接词 / 剂型描述词
IGNORED_WORDS = {"and", "or", "solution"}

# USAN（美）与 INN（国际）同药异拼：同组内的写法视为同一个名字
SPELLING_VARIANTS = [
    {"cephalexin", "cefalexin"},
    {"torsemide", "torasemide"},
]

# 任一变体 → 该组规范写法（组内字典序最小者），两边都要过一遍才能比较
VARIANT_CANONICAL = {
    variant: min(group) for group in SPELLING_VARIANTS for variant in group
}

DRUG_ID_RE = re.compile(r"^drug_\d{3}$")


def squash(text):
    """小写、去所有格符号、非字母数字转下划线、去掉连接词与剂型词。"""
    text = text.lower().replace("'", "")
    text = re.sub(r"[^a-z0-9]+", " ", text)
    return "_".join(w for w in text.split() if w not in IGNORED_WORDS)


def apply_spelling_variant(name):
    return VARIANT_CANONICAL.get(name, name)


def canonical_name(name_en):
    """主名（括号里的商品名/别名不参与），用于判断重名。"""
    return apply_spelling_variant(squash(re.sub(r"\([^)]*\)", " ", name_en)))


def name_aliases(name_en):
    """该 name_en 可接受的全部写法：主名、括号别名、"/"并列成分、首字母缩略词。"""
    aliases = set()

    def add(text):
        squashed = apply_spelling_variant(squash(text))
        if not squashed:
            return
        aliases.add(squashed)
        words = squashed.split("_")
        if len(words) >= 2:
            aliases.add("".join(w[0] for w in words))

    for segment in re.sub(r"\(([^)]*)\)", r" / \1 ", name_en).split("/"):
        add(segment)
    return aliases


def disease_drug_refs(diseases):
    """统计 diseases[].drugs[].drug 对各 drug id 的引用次数。"""
    counts = Counter()

    def walk(node):
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "drug" and isinstance(value, str) and DRUG_ID_RE.match(value):
                    counts[value] += 1
                else:
                    walk(value)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    walk(diseases)
    return counts


def main():
    knowledge = json.loads(KNOWLEDGE_PATH.read_text(encoding="utf-8"))
    drug_map = json.loads(MAP_PATH.read_text(encoding="utf-8"))

    drugs = knowledge["drugs"]
    by_id = {d["id"]: d for d in drugs}
    interactions = knowledge.get("drug_interactions", [])

    failures = []

    # (a) 映射表引用的 id 必须存在
    missing_ids = sorted(
        f"{key} → {drug_id}（knowledge.json 中不存在）"
        for key, drug_id in drug_map.items()
        if drug_id not in by_id
    )
    if missing_ids:
        failures.append(("映射表指向不存在的 drug id", missing_ids))

    # (b) 键的规范英文名与所指药物 name_en 一致
    name_mismatches = []
    for key, drug_id in sorted(drug_map.items()):
        drug = by_id.get(drug_id)
        if drug is None:
            continue  # 已由 (a) 报告
        if apply_spelling_variant(key) not in name_aliases(drug["name_en"]):
            name_mismatches.append(
                f"{key} → {drug_id} name_en={drug['name_en']!r} name_zh={drug.get('name_zh')!r}"
            )
    if name_mismatches:
        failures.append(("映射表键与所指药物的英文名不一致", name_mismatches))

    # (c) drugs 数组内不存在规范英文名重复
    groups = defaultdict(list)
    for drug in drugs:
        groups[canonical_name(drug["name_en"])].append(drug)
    ref_counts = disease_drug_refs(knowledge.get("diseases", []))
    interaction_counts = Counter()
    for row in interactions:
        interaction_counts[row.get("drug_a", "")] += 1
        interaction_counts[row.get("drug_b", "")] += 1

    duplicate_lines = []
    for canon, members in sorted(groups.items()):
        if len(members) < 2:
            continue
        members.sort(key=lambda d: d["id"])
        kept, dropped = members[0], members[1:]
        detail = "，".join(
            f"{d['id']}({d.get('name_zh')}) 被治疗方案引 {ref_counts.get(d['id'], 0)} 次/"
            f"相互作用引 {interaction_counts.get(d['id'], 0)} 次"
            for d in dropped
        )
        duplicate_lines.append(
            f"{canon}: 保留 {kept['id']}({kept.get('name_zh')})；弃用 {detail}"
        )
    if duplicate_lines:
        failures.append(
            ("drugs 数组存在规范英文名重复（游戏名→id 无法唯一，相互作用会漏检）", duplicate_lines)
        )

    # (d) drug_interactions 引用的 id 都存在
    dangling = [
        f"#{i} {row['drug_a']} ↔ {row['drug_b']} ({row.get('severity')})"
        for i, row in enumerate(interactions)
        if row.get("drug_a") not in by_id or row.get("drug_b") not in by_id
    ]
    if dangling:
        failures.append(("drug_interactions 指向不存在的 drug id", dangling))

    print(f"knowledge.json: {len(drugs)} 条药物，{len(interactions)} 条相互作用")
    print(f"game_drug_to_vk_id.json: {len(drug_map)} 个键")
    print()

    if not failures:
        print("PASS: (a)(b)(c)(d) 全部通过")
        return 0

    for title, lines in failures:
        print(f"FAIL {title}:")
        for line in lines:
            print(f"  - {line}")
    print()
    print(f"FAILED: {len(failures)} 项断言不通过（共 4 项）")
    return 1


if __name__ == "__main__":
    sys.exit(main())
