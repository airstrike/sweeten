#!/usr/bin/env python3
"""Walk target/criterion/ and emit a markdown summary pairing sweeten
and iced benches, with iced expressed as a multiplier of sweeten.

Three modes:

    python3 bench_summary.py                  # render current target/criterion/
    python3 bench_summary.py capture <stage>  # snapshot into benches/history.json
    python3 bench_summary.py history          # render multi-stage progression

The history file lives at benches/history.json. Each capture appends a
new stage entry (or replaces an existing one with the same name).
Mean point estimates are stored as raw nanoseconds; multipliers are
computed at render time.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent
CRITERION = ROOT / "target" / "criterion"
HISTORY = ROOT / "benches" / "history.json"
IMPLS = ("sweeten", "iced")


# --------------------------------------------------------------------- helpers

def fmt_time(ns: float) -> str:
    """Pick ns / µs / ms / s based on magnitude."""
    if ns < 1_000:
        return f"{ns:.1f} ns"
    if ns < 1_000_000:
        return f"{ns / 1_000:.2f} µs"
    if ns < 1_000_000_000:
        return f"{ns / 1_000_000:.2f} ms"
    return f"{ns / 1_000_000_000:.2f} s"


def numeric_key(s: str) -> tuple[int, float, str]:
    """Sort key: numbers numerically, then anything else alphabetically.

    Handles plain ints ("100"), depth-fanout codes ("d10_f3"), and
    arbitrary strings ("center")."""
    if not s:
        return (0, 0.0, "")
    try:
        return (1, float(s), s)
    except ValueError:
        m = re.match(r"d(\d+)_f(\d+)", s)
        if m:
            return (1, int(m.group(1)) * 100 + int(m.group(2)), s)
        return (2, 0.0, s)


def collect():
    """Returns {group: {param: {impl: ns}}}.

    `param` is "" for non-parameterized benches. `impl` ∈ {sweeten,
    iced, "_"} — non-impl benches (e.g. reverse_50/forward) go under
    "_" so they still render in the table.
    """
    if not CRITERION.exists():
        raise SystemExit(
            f"no criterion output at {CRITERION}; run `cargo bench` first"
        )

    out: dict[str, dict[str, dict[str, float]]] = {}

    for est in CRITERION.rglob("new/estimates.json"):
        rel = est.relative_to(CRITERION).parts
        if len(rel) < 4:
            continue
        group = rel[0]
        bench_parts = rel[1:-2]

        if "report" in bench_parts:
            continue

        if len(bench_parts) == 1:
            impl = bench_parts[0] if bench_parts[0] in IMPLS else "_"
            param = "" if bench_parts[0] in IMPLS else bench_parts[0]
        elif len(bench_parts) == 2:
            impl, param = bench_parts
            if impl not in IMPLS:
                impl = "_"
                param = "/".join(bench_parts)
        else:
            continue

        try:
            with est.open() as f:
                ns = json.load(f)["mean"]["point_estimate"]
        except (KeyError, json.JSONDecodeError):
            continue

        out.setdefault(group, {}).setdefault(param, {})[impl] = ns

    return out


def current_commit() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=ROOT,
            text=True,
        ).strip()
    except subprocess.CalledProcessError:
        return "unknown"


# --------------------------------------------------------------------- render: single

def render_single(data) -> str:
    """Render the current criterion run."""
    lines: list[str] = []
    lines.append("# flex_vs_iced — bench summary")
    lines.append("")
    lines.append(
        "Sweeten = **1.00×** baseline. "
        "Iced × > 1 means iced is slower; < 1 means iced is faster. "
        "Times are mean point estimates from criterion."
    )
    lines.append("")

    for group in sorted(data.keys()):
        rows = data[group]
        has_iced = any("iced" in r for r in rows.values())

        lines.append(f"## {group}")
        lines.append("")

        if has_iced:
            lines.append("| param | sweeten | iced | iced × |")
            lines.append("|---|---|---|---|")
        else:
            lines.append("| param | sweeten |")
            lines.append("|---|---|")

        for param in sorted(rows.keys(), key=numeric_key):
            cells = rows[param]
            sw = cells.get("sweeten")
            ic = cells.get("iced")
            other = cells.get("_")
            if sw is None and other is not None:
                sw = other

            sw_s = fmt_time(sw) if sw is not None else "—"
            param_disp = param if param else "(default)"

            if has_iced:
                ic_s = fmt_time(ic) if ic is not None else "—"
                if sw is not None and ic is not None:
                    ratio = f"{ic / sw:.2f}×"
                else:
                    ratio = "—"
                lines.append(f"| {param_disp} | {sw_s} | {ic_s} | {ratio} |")
            else:
                lines.append(f"| {param_disp} | {sw_s} |")

        lines.append("")

    return "\n".join(lines)


# --------------------------------------------------------------------- capture

def capture(stage_name: str) -> None:
    """Snapshot current criterion data into history.json under <stage_name>."""
    data = collect()

    history = load_history()
    stages = history.setdefault("stages", [])

    # Replace any prior entry with the same name; otherwise append.
    stages = [s for s in stages if s.get("name") != stage_name]
    stages.append(
        {
            "name": stage_name,
            "captured_at": datetime.now(timezone.utc)
            .isoformat(timespec="seconds")
            .replace("+00:00", "Z"),
            "commit": current_commit(),
            "results": data,
        }
    )
    history["stages"] = stages

    HISTORY.parent.mkdir(parents=True, exist_ok=True)
    HISTORY.write_text(json.dumps(history, indent=2) + "\n")
    print(
        f"captured stage '{stage_name}' "
        f"({sum(len(g) for g in data.values())} bench rows) "
        f"→ {HISTORY.relative_to(ROOT)}",
        file=sys.stderr,
    )


def load_history() -> dict:
    if HISTORY.exists():
        return json.loads(HISTORY.read_text())
    return {}


# --------------------------------------------------------------------- render: history

def render_history() -> str:
    """Render the multi-stage progression from history.json."""
    history = load_history()
    stages = history.get("stages", [])
    if not stages:
        raise SystemExit(
            f"no stages in {HISTORY.relative_to(ROOT)}; "
            f"run `bench_summary.py capture <stage>` first"
        )

    stage_names = [s["name"] for s in stages]
    by_stage = {s["name"]: s["results"] for s in stages}
    baseline_name = stage_names[0]
    final_name = stage_names[-1]

    # Union of (group, param) keys across all stages.
    groups: dict[str, set[str]] = {}
    for s in stages:
        for g, params in s["results"].items():
            groups.setdefault(g, set()).update(params.keys())

    lines: list[str] = []
    lines.append("# flex_vs_iced — optimisation progression")
    lines.append("")

    lines.append("Stages captured:")
    for s in stages:
        lines.append(f"- **{s['name']}** — {s['captured_at']} @ `{s['commit']}`")
    lines.append("")
    lines.append(
        f"`Δ sweeten` = {final_name} / {baseline_name} (lower is better). "
        f"`vs iced` = {final_name} sweeten / {final_name} iced (lower means sweeten faster)."
    )
    lines.append("")

    for group in sorted(groups.keys()):
        rows = groups[group]
        # Does any stage have iced data for this group?
        has_iced = any(
            "iced" in by_stage[st].get(group, {}).get(p, {})
            for st in stage_names
            for p in rows
        )

        lines.append(f"## {group}")
        lines.append("")

        header = ["param"]
        for st in stage_names:
            header.append(f"sweeten[{st}]")
        if has_iced:
            header.append(f"iced[{final_name}]")
        header.append(f"Δ sweeten")
        if has_iced:
            header.append(f"vs iced")

        lines.append("| " + " | ".join(header) + " |")
        lines.append("|" + "|".join("---" for _ in header) + "|")

        for param in sorted(rows, key=numeric_key):
            param_disp = param if param else "(default)"
            row = [param_disp]

            # Per-stage sweeten cells.
            sw_per_stage: dict[str, float | None] = {}
            for st in stage_names:
                cells = by_stage[st].get(group, {}).get(param, {})
                sw = cells.get("sweeten") or cells.get("_")
                sw_per_stage[st] = sw
                row.append(fmt_time(sw) if sw is not None else "—")

            # Final iced cell.
            ic_final = (
                by_stage[final_name]
                .get(group, {})
                .get(param, {})
                .get("iced")
            )
            if has_iced:
                row.append(fmt_time(ic_final) if ic_final is not None else "—")

            # Δ sweeten = final / baseline.
            sw_base = sw_per_stage[baseline_name]
            sw_final = sw_per_stage[final_name]
            if sw_base and sw_final:
                row.append(f"{sw_final / sw_base:.2f}×")
            else:
                row.append("—")

            # vs iced = final_sweeten / final_iced.
            if has_iced:
                if sw_final and ic_final:
                    row.append(f"{sw_final / ic_final:.2f}×")
                else:
                    row.append("—")

            lines.append("| " + " | ".join(row) + " |")

        lines.append("")

    return "\n".join(lines)


# --------------------------------------------------------------------- entry

def main() -> None:
    args = sys.argv[1:]
    if not args:
        print(render_single(collect()))
        return

    cmd, *rest = args
    if cmd == "capture":
        if len(rest) != 1:
            raise SystemExit("usage: bench_summary.py capture <stage-name>")
        capture(rest[0])
        return
    if cmd == "history":
        print(render_history())
        return

    raise SystemExit(
        "usage:\n"
        "  bench_summary.py                  # render current run\n"
        "  bench_summary.py capture <stage>  # snapshot into history.json\n"
        "  bench_summary.py history          # render progression"
    )


if __name__ == "__main__":
    main()
