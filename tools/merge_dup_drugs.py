#!/usr/bin/env python3
"""一次性迁移：合并 10 组重名药物（2026-10-19 签字执行）。

用法：python tools/merge_dup_drugs.py            # 执行
      python tools/merge_dup_drugs.py --check    # 只校验不写盘

临床文字为人工逐条拟定（依据 docs/drug-dup-unique-content.md 的独有信息清单），
本脚本只负责精确施加与自动重定向引用；每处替换都断言命中，不允许静默跳过。
幂等保护：若弃用 id 已不存在则直接退出。
"""
import json
import sys
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
KB = ROOT / "data" / "knowledge.json"

# 保留 id ← 弃用 id
PAIRS = {
    "drug_001": "drug_078",  # 阿莫西林-克拉维酸
    "drug_010": "drug_051",  # 马罗匹坦
    "drug_011": "drug_053",  # 曲马多
    "drug_016": "drug_042",  # 苯巴比妥
    "drug_017": "drug_043",  # 溴化钾
    "drug_021": "drug_085",  # 特比萘芬
    "drug_022": "drug_060",  # 伊曲康唑（无独有内容）
    "drug_024": "drug_057",  # 阿替美唑（弃用项中文名"阿替帕唑"有误，无独有内容）
    "drug_032": "drug_052",  # 加巴喷丁（无独有内容）
    "drug_038": "drug_055",  # 贝那普利（无独有内容）
}

# 追加到 markdown 列表字段（"- 项" 形式，已存在同文本则跳过）
APPEND = [
    ("drug_001", "contraindications", ["严重肝功能不全"]),
    ("drug_010", "side_effects", ["一过性心动过缓（SC）"]),
    ("drug_011", "side_effects", ["猫可能产生兴奋而非镇静（矛盾性兴奋）"]),
    ("drug_016", "indications", ["猫癫痫（二线用药）"]),
    ("drug_016", "contraindications", ["妊娠", "与氯丙嗪合用（降低惊厥阈值）"]),
    ("drug_017", "indications", ["犬难治性癫痫（与苯巴比妥联合）"]),
    ("drug_017", "contraindications", ["妊娠", "与含溴药物合用"]),
    ("drug_017", "side_effects", ["虚弱（初期，1-2 周后耐受）", "胃肠道反应（恶心、呕吐）"]),
    ("drug_021", "contraindications", ["妊娠"]),
]

# 精确子串替换：(id, field, old, new)。old 必须出现且仅按原样匹配一次。
REPLACE = [
    # 疗程信息（弃用项独有）
    ("drug_001", "species_dosing",
     "**犬**: dose=12.5-25 mg/kg, route=PO, frequency=q12h",
     "**犬**: dose=12.5-25 mg/kg, route=PO, frequency=q12h（连用 5-7 天）"),
    # 曲马多：适应症覆盖轻度疼痛
    ("drug_011", "indications", "- 中度疼痛", "- 轻至中度疼痛"),
    # 曲马多：猫疗效不确切
    ("drug_011", "species_dosing", "（猫代谢个体差异大）", "（猫代谢个体差异大，疗效不确切）"),
    # 苯巴比妥：点名再生障碍性贫血
    ("drug_016", "side_effects", "- 血液学毒性（罕见）",
     "- 血液学毒性（罕见，含再生障碍性贫血）"),
    # 苯巴比妥：癫痫持续状态 IV 抢救剂量（弃用项独有）
    ("drug_016", "species_dosing",
     "**犬**: dose=2-5 mg/kg, route=PO, frequency=q12h（需监测血药浓度）",
     "**犬**: dose=2-5 mg/kg, route=PO, frequency=q12h（需监测血药浓度）；"
     "癫痫持续状态 20 mg/kg IV"),
    ("drug_016", "species_dosing",
     "**猫**: dose=2-3 mg/kg, route=PO, frequency=q12h",
     "**猫**: dose=2-3 mg/kg, route=PO, frequency=q12h（猫二线用药）"),
    # 溴化钾：负荷剂量给出具体方案（弃用项独有），并保留"可加倍"的口径说明
    ("drug_017", "species_dosing", "（负荷剂量可加倍）",
     "（负荷剂量 200-300 mg/kg，分 5 天给予）"),
    # 特比萘芬：疗程周数（弃用项独有）
    ("drug_021", "species_dosing",
     "**猫**: dose=20-40 mg/kg, route=PO, frequency=q24h（猫对疗程要求更长）",
     "**猫**: dose=20-40 mg/kg, route=PO, frequency=q24h（猫对疗程要求更长）\n**疗程**: 4-12 周"),
    # ── 冲突裁定：马罗匹坦药动学，弃用项数值经文献核对更准，故覆盖保留项 ──
    # 依据：犬血浆蛋白结合率实测 >99%；皮下 1 mg/kg 后 t1/2 约 7.75 h（口服 2 mg/kg 约 4.03 h）
    ("drug_010", "pk_pd", "血浆蛋白结合率>95%", "血浆蛋白结合率>99%"),
    ("drug_010", "pk_pd", "半衰期犬约8-12小时",
     "半衰期犬约 8 小时（皮下 1 mg/kg 后约 7.75 h，口服 2 mg/kg 后约 4 h）"),
    # 溴化钾：多饮多尿补上"多食"
    ("drug_017", "side_effects", "- 多饮多尿", "- 多饮多尿多食"),
]

# 追加到散文式段落字段（adverse_mechanism 等）
APPEND_PROSE = [
    ("drug_010", "adverse_mechanism", "罕见过敏反应（荨麻疹/面部肿胀）可见，发生率极低。"),
    ("drug_017", "adverse_mechanism",
     "虚弱（初期常见，通常 1-2 周后耐受）与中枢抑制及电解质分布有关。"),
]

# 明确丢弃的弃用项内容（记录理由，避免以后重复讨论）
DISCARDED = [
    ("drug_043", "猫禁用理由「耐受差，可致支气管痉挛」",
     "与保留项「猫溴化物可致致命性肺炎」冲突；猫用溴化物的经典不良反应为伪过敏性/嗜酸性肺炎（可致命），"
     "支气管痉挛不是标准机制，故保留肺炎说法。"),
    ("drug_085", "drug_class「烯丙胺类抗真菌药」",
     "与保留项「丙烯胺类」为同一类药（allylamine）的不同译法，保留原译法。"),
    ("drug_078", "indications「口腔感染」",
     "保留项已列「牙周感染」，含义等价。"),
    ("drug_057", "name_zh「阿替帕唑」",
     "弃用项中文名有误（atipamezole 应作阿替美唑），保留项正确。"),
]


# 引用重定向方向：弃用 id → 保留 id（PAIRS 是 保留→弃用，这里必须反转，
# 否则会把指向保留项的引用改到即将删除的弃用项上，造成悬空引用）
REDIRECT = {drop: keep for keep, drop in PAIRS.items()}


def load():
    return json.loads(KB.read_text(encoding="utf-8"))


def norm_items(value):
    """把字段值统一成 list[str]，兼容 markdown 字符串与 JSON 数组两种历史格式。"""
    if value is None:
        return []
    if isinstance(value, list):
        return [str(v).strip() for v in value if str(v).strip()]
    return [ln.strip() for ln in str(value).splitlines() if ln.strip()]


def to_markdown(items):
    return "\n".join(items)


def apply_merge(data):
    drugs = {x["id"]: x for x in data["drugs"]}
    missing = [d for d in PAIRS.values() if d not in drugs]
    if missing:
        print(f"已合并过，弃用 id 不存在：{missing}，跳过。")
        return data, []
    log = []

    for keep_id, field, items in APPEND:
        rec = drugs[keep_id]
        cur = norm_items(rec.get(field))
        for it in items:
            line = it if it.startswith("-") else f"- {it}"
            bare = line.lstrip("- ").strip()
            if any(bare == c.lstrip("- ").strip() for c in cur):
                log.append(f"  (已存在，跳过) {keep_id}.{field} += {bare}")
                continue
            cur.append(line)
            log.append(f"  {keep_id}.{field} += {bare}")
        rec[field] = to_markdown(cur)

    for keep_id, field, old, new in REPLACE:
        rec = drugs[keep_id]
        val = rec.get(field)
        if not isinstance(val, str):
            raise SystemExit(f"断言失败：{keep_id}.{field} 不是字符串，无法替换")
        n = val.count(old)
        if n != 1:
            raise SystemExit(f"断言失败：{keep_id}.{field} 中 {old!r} 出现 {n} 次（需恰好 1 次）")
        rec[field] = val.replace(old, new)
        log.append(f"  {keep_id}.{field} 替换：{old[:28]}… → {new[:28]}…")

    for keep_id, field, sentence in APPEND_PROSE:
        rec = drugs[keep_id]
        val = rec.get(field) or ""
        if sentence in val:
            continue
        rec[field] = (val.rstrip() + " " + sentence) if val else sentence
        log.append(f"  {keep_id}.{field} += 段落一句")

    drop_ids = set(PAIRS.values())
    before = len(data["drugs"])
    data["drugs"] = [x for x in data["drugs"] if x["id"] not in drop_ids]
    log.append(f"  删除弃用记录 {before} → {len(data['drugs'])} 条")

    # ── 引用重定向：drug_interactions ──
    redirect = 0
    seen = {}
    new_inter = []
    for it in data.get("drug_interactions", []):
        a, b = it["drug_a"], it["drug_b"]
        ra, rb = REDIRECT.get(a, a), REDIRECT.get(b, b)
        if ra == rb:
            raise SystemExit(f"重定向后自相互作用，需人工处理：{it}")
        if (ra, rb) != (a, b):
            redirect += 1
            it = dict(it, drug_a=ra, drug_b=rb)
        key = frozenset((ra, rb))
        if key in seen:
            log.append(f"  相互作用去重：{ra}↔{rb}（保留先出现的描述）")
            continue
        seen[key] = True
        new_inter.append(it)
    data["drug_interactions"] = new_inter
    log.append(f"  drug_interactions 重定向 {redirect} 处，现存 {len(new_inter)} 条")

    # ── 引用重定向：diseases[].drugs[].drug ──
    rep = 0
    dedup = 0
    for dis in data.get("diseases", []):
        kept_rows = []
        for row in dis.get("drugs", []) or []:
            did = row.get("drug")
            if did in REDIRECT:
                row = dict(row, drug=REDIRECT[did])
                rep += 1
            key = (row.get("drug"), row.get("line"), row.get("species"))
            dup = next((k for k in kept_rows
                        if (k.get("drug"), k.get("line"), k.get("species")) == key), None)
            if dup is not None:
                dedup += 1
                if len(str(row.get("notes") or "")) > len(str(dup.get("notes") or "")):
                    kept_rows[kept_rows.index(dup)] = row
                log.append(f"  {dis['id']} 治疗方案去重：{key}")
                continue
            kept_rows.append(row)
        if dis.get("drugs") is not None:
            dis["drugs"] = kept_rows
    log.append(f"  diseases[].drugs 重定向 {rep} 处，去重 {dedup} 处")
    return data, log


def main():
    check = "--check" in sys.argv
    data = load()
    data, log = apply_merge(data)
    print("\n".join(log) or "(无改动)")
    for rid, item, why in DISCARDED:
        print(f"  [丢弃并记录] {rid}: {item} —— {why}")
    if check:
        print("--check 模式，未写盘")
        return
    KB.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"已写盘 {KB}")


if __name__ == "__main__":
    main()
