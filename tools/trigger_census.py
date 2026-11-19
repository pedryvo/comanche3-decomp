"""Inventory trigger syntax in mission files extracted from RESOURCE.RES."""

import argparse
from collections import Counter
from pathlib import Path
import re


OPERATORS = {"and", "or", "andn"}
CONTROL_WORDS = OPERATORS | {"if", "then", "<"}


def read_rules(path):
    rules = []
    state = "outside"
    for number, raw in enumerate(path.read_text(encoding="latin-1").splitlines(), 1):
        marker = raw.lower()
        if "trigger data begins" in marker and state == "outside":
            state = "if"
            continue
        if "trigger data ends" in marker and state == "end marker":
            return rules
        line = raw.partition(";")[0].strip()
        if not line or state == "outside":
            continue
        opcode = line.split(",", 1)[0].strip().lower()
        if state == "if" and line.lower() == "if":
            conditions, operators = [], []
            state = "condition"
        elif state == "if" and line == "<":
            state = "end marker"
        elif state == "condition" and opcode not in CONTROL_WORDS:
            conditions.append(opcode)
            state = "operator"
        elif state == "operator" and line.lower() == "then":
            state = "action"
        elif state == "operator" and line.lower() in OPERATORS:
            operators.append(opcode)
            state = "condition"
        elif state == "action" and opcode not in CONTROL_WORDS:
            rules.append((tuple(conditions), tuple(operators), opcode))
            state = "if"
        else:
            raise ValueError(f"{path}:{number}: expected {state}, found {line!r}")
    raise ValueError(f"{path}: unexpected EOF in state {state}")


def summarize(label, files):
    missions = {path.name: read_rules(path) for path in files}
    rules = [rule for mission_rules in missions.values() for rule in mission_rules]
    print(f"{label}: {len(missions)} files, {len(rules)} rules")
    for index, name in enumerate(("conditions", "operators", "actions")):
        counts = Counter()
        for rule in rules:
            counts.update((rule[index],) if index == 2 else rule[index])
        print(f"  {name} ({len(counts)}): {counts.most_common()}")
    print("  missions with win:", sum(any(rule[2] == "win" for rule in rs) for rs in missions.values()))
    print("  empty trigger blocks:", [name for name, rs in missions.items() if not rs])
    print("  largest condition chain:", max((len(rule[0]) for rule in rules), default=0))
    return missions


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    files = sorted(p for p in args.directory.iterdir() if p.suffix.lower() == ".mis")
    if not files:
        parser.error("no .mis files found")
    campaign = [path for path in files if re.fullmatch(r"c\d+m\d+", path.stem.lower())]
    missions = summarize("Campaign-named files", campaign)
    summarize("Other files", [path for path in files if path not in campaign])

    sample = missions.get("c1m3.mis")
    if sample:
        vocabulary = [set().union(*(rule[i] for rule in sample)) for i in range(2)]
        actions = {rule[2] for rule in sample}
        compatible = [
            name for name, rules in missions.items()
            if rules and all(set(conditions) <= vocabulary[0]
                   and set(operators) <= vocabulary[1]
                   and action in actions for conditions, operators, action in rules)
        ]
        nonempty = sum(bool(rules) for rules in missions.values())
        print(f"Nonempty campaign files using only c1m3 vocabulary: {len(compatible)}/{nonempty}")
        print(" ", ", ".join(compatible))
        print("c1m3 win rules:", [rule for rule in sample if rule[2] == "win"])


if __name__ == "__main__":
    main()
