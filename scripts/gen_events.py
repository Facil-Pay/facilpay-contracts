#!/usr/bin/env python3
"""Regenerate docs/EVENTS.md from the `#[contractevent]` structs in each contract.

Usage (from the repository root):

    python3 scripts/gen_events.py            # rewrites docs/EVENTS.md
    python3 scripts/gen_events.py --stdout   # prints the result instead

The script only reads source files. For every `#[contractevent]` struct in
`contracts/<name>/src/lib.rs` it records:

* the topic symbol: the explicit `topics = ["..."]` value if there is one,
  otherwise the struct name in snake_case (soroban-sdk's default);
* the payload fields, in declaration order;
* the functions that build the event. When an event is built inside a
  private helper, the public functions that reach that helper are listed.

Inline `#[cfg(test)] mod ... { ... }` blocks are ignored, as are the separate
test files, since only `lib.rs` is scanned.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "EVENTS.md"

CONTRACTS = [
    ("Admin", "admin"),
    ("Escrow", "escrow"),
    ("Payment", "payment"),
    ("Refund", "refund"),
]

EVENT_RE = re.compile(
    r"#\[contractevent(?:\(\s*topics\s*=\s*\[\s*\"(?P<topic>[^\"]+)\"\s*\]\s*\))?\]"
    r"(?:\s*#\[[^\]]*\])*"
    r"\s*pub\s+struct\s+(?P<name>\w+)\s*\{(?P<body>[^}]*)\}",
    re.S,
)
FIELD_RE = re.compile(r"^\s*pub\s+(\w+)\s*:\s*(.+?),?\s*$")
FN_RE = re.compile(r"^\s*(?P<pub>pub(?:\([^)]*\))?\s+)?fn\s+(?P<name>\w+)")
TEST_MOD_RE = re.compile(r"#\[cfg\(test\)\]\s*\n\s*(?:pub\s+)?mod\s+\w+\s*\{")


def snake_case(name):
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s)
    return s.lower()


def strip_inline_test_modules(src):
    """Blank out `#[cfg(test)] mod x { ... }` blocks, keeping line numbers."""
    while True:
        m = TEST_MOD_RE.search(src)
        if not m:
            return src
        depth = 0
        end = len(src)
        for i in range(m.end() - 1, len(src)):
            if src[i] == "{":
                depth += 1
            elif src[i] == "}":
                depth -= 1
                if depth == 0:
                    end = i + 1
                    break
        blank = re.sub(r"[^\n]", " ", src[m.start():end])
        src = src[: m.start()] + blank + src[end:]


def strip_comments(line):
    return line.split("//", 1)[0]


def function_index(lines):
    """Return, per line, the (name, is_pub) of the enclosing fn, plus all fns."""
    current = None
    owner = []
    fns = {}
    for line in lines:
        m = FN_RE.match(line)
        if m:
            current = (m.group("name"), bool(m.group("pub")))
            fns.setdefault(current[0], current[1])
        owner.append(current)
    return owner, fns


def public_callers(helper, lines, owner, fns, seen=None):
    """Public fns that reach private fn `helper`, following private callers."""
    seen = seen or set()
    if helper in seen:
        return set()
    seen.add(helper)
    call = re.compile(r"(?:Self::|\b)" + re.escape(helper) + r"\s*\(")
    result = set()
    for i, line in enumerate(lines):
        code = strip_comments(line)
        if FN_RE.match(code) or not call.search(code):
            continue
        fn = owner[i]
        if fn is None or fn[0] == helper:
            continue
        if fn[1]:
            result.add(fn[0])
        else:
            result |= public_callers(fn[0], lines, owner, fns, seen)
    return result


def collect(contract_dir):
    path = ROOT / "contracts" / contract_dir / "src" / "lib.rs"
    src = strip_inline_test_modules(path.read_text())
    lines = src.split("\n")
    owner, fns = function_index(lines)

    events = []
    for m in EVENT_RE.finditer(src):
        name = m.group("name")
        explicit = m.group("topic")
        fields = []
        for raw in m.group("body").split("\n"):
            fm = FIELD_RE.match(strip_comments(raw))
            if fm:
                fields.append((fm.group(1), fm.group(2).strip()))

        build = re.compile(r"(?<![\w:])" + re.escape(name) + r"\s*\{")
        direct, via = set(), {}
        for i, line in enumerate(lines):
            code = strip_comments(line)
            if not build.search(code) or re.search(r"\bstruct\s+" + name + r"\b", code):
                continue
            fn = owner[i]
            if fn is None:
                continue
            if fn[1]:
                direct.add(fn[0])
            else:
                via[fn[0]] = public_callers(fn[0], lines, owner, fns)

        events.append(
            {
                "name": name,
                "topic": explicit or snake_case(name),
                "explicit": explicit is not None and explicit != snake_case(name),
                "fields": fields,
                "direct": sorted(direct),
                "via": dict(sorted(via.items())),
            }
        )
    return events


def code(s):
    return "`" + s.replace("|", "\\|") + "`"


def emitted_by(ev):
    parts = [code(f) for f in ev["direct"]]
    for helper, callers in ev["via"].items():
        if callers:
            parts.append(
                code(helper) + " (internal, used by "
                + ", ".join(code(c) for c in sorted(callers)) + ")"
            )
        else:
            parts.append(code(helper) + " (internal)")
    return ", ".join(parts) if parts else "_not emitted_"


def render():
    all_events = [(title, d, collect(d)) for title, d in CONTRACTS]
    total = sum(len(e) for _, _, e in all_events)
    out = []
    w = out.append
    w("# Contract Event Reference")
    w("")
    w("<!-- Generated by scripts/gen_events.py. Do not edit by hand; rerun the script instead. -->")
    w("")
    w("Every `#[contractevent]` struct in the FacilPay contracts, for indexer and backend developers.")
    w(f"There are {total} events across {len(all_events)} contracts.")
    w("")
    w("## How events are encoded")
    w("")
    w("All events are declared with soroban-sdk's `#[contractevent]` macro and published with `.publish(&env)`.")
    w("")
    w("- **Topics:** a single `Symbol`, listed in the *Topic* column below. By default it is the struct")
    w("  name in snake_case (`EscrowReleased` → `escrow_released`). A struct can set it explicitly with")
    w("  `#[contractevent(topics = [\"...\"])]`; those are listed in")
    w("  [Events with an explicit topic](#events-with-an-explicit-topic). No event uses `#[topic]` fields,")
    w("  so the topic list never has more than this one symbol.")
    w("- **Data:** a map keyed by field name, holding every field in the *Payload fields* column.")
    w("- **Emitted by:** the contract functions that build the event. When the event is built inside a")
    w("  private helper, the helper is named along with the public functions that call it. A public")
    w("  function that calls another public function (for example `enforce_processing_deadline` calling")
    w("  `check_processing_deadline`) also emits that function's events but is not listed separately.")
    w("  _not emitted_ means the struct is declared but no code in `lib.rs` currently publishes it.")
    w("")
    w("Event names that appear in more than one contract (for example `ContractUpgraded`) are separate")
    w("events: filter on the emitting contract ID as well as the topic.")
    w("")
    w("## Events with an explicit topic")
    w("")
    w("These events set their topic with `#[contractevent(topics = [...])]`, so the topic is **not** the")
    w("snake_case struct name. Index them by the topic shown here.")
    w("")
    w("| Contract | Struct | Topic | Default topic it replaces |")
    w("| --- | --- | --- | --- |")
    for title, _, events in all_events:
        for ev in events:
            if ev["explicit"]:
                w(f"| {title} | {code(ev['name'])} | {code(ev['topic'])} | {code(snake_case(ev['name']))} |")
    w("")
    w("## Contents")
    w("")
    for title, d, events in all_events:
        w(f"- [{title} contract](#{title.lower()}-contract) ({len(events)} events)")
    w("")
    for title, d, events in all_events:
        w(f"## {title} contract")
        w("")
        w(f"Source: [`contracts/{d}/src/lib.rs`](../contracts/{d}/src/lib.rs).")
        w("")
        w("| Struct | Topic | Payload fields | Emitted by |")
        w("| --- | --- | --- | --- |")
        for ev in sorted(events, key=lambda e: e["name"]):
            topic = code(ev["topic"]) + (" **(explicit)**" if ev["explicit"] else "")
            fields = "<br>".join(code(f"{n}: {t}") for n, t in ev["fields"]) or "—"
            w(f"| {code(ev['name'])} | {topic} | {fields} | {emitted_by(ev)} |")
        w("")
    w("## Regenerating this file")
    w("")
    w("```bash")
    w("python3 scripts/gen_events.py")
    w("```")
    w("")
    w("Quick check without Python, listing every event struct with any explicit topic:")
    w("")
    w("```bash")
    w("grep -n -A3 '#\\[contractevent' contracts/*/src/lib.rs | grep -E 'contractevent|pub struct'")
    w("# only the events whose topic is set explicitly:")
    w("grep -n -A3 '#\\[contractevent(' contracts/*/src/lib.rs | grep -E 'topics|pub struct'")
    w("```")
    w("")
    return "\n".join(out)


def main():
    text = render()
    if "--stdout" in sys.argv:
        sys.stdout.write(text)
    else:
        OUT.write_text(text)
        print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
