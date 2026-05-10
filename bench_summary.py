#!/usr/bin/env python3
"""Walk target/criterion/ and emit a markdown summary pairing sweeten
and iced benches, with iced expressed as a multiplier of sweeten.

Run after `cargo bench --bench flex_vs_iced`. Writes to stdout.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Optional

ROOT = Path(__file__).resolve().parent / "target" / "criterion"
IMPLS = ("sweeten", "iced")


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
    iced, <other>} — non-impl benches go under <other> for completeness.
    """
    if not ROOT.exists():
        raise SystemExit(
            f"no criterion output at {ROOT}; run `cargo bench` first"
        )

    out: dict[str, dict[str, dict[str, float]]] = {}

    for est in ROOT.rglob("new/estimates.json"):
        rel = est.relative_to(ROOT).parts
        # rel: (<group>, *bench-id-parts*, "new", "estimates.json")
        if len(rel) < 4:
            continue
        group = rel[0]
        bench_parts = rel[1:-2]  # drop "new", "estimates.json"

        # Skip the "report" pseudo-dir if it ever sneaks in.
        if "report" in bench_parts:
            continue

        if len(bench_parts) == 1:
            # Either a non-parameterized leaf ("sweeten") or a leaf in
            # a sweeten-only group with no impl prefix ("forward").
            impl = bench_parts[0] if bench_parts[0] in IMPLS else "_"
            param = "" if bench_parts[0] in IMPLS else bench_parts[0]
        elif len(bench_parts) == 2:
            # Parameterized leaf: <impl>/<param>.
            impl, param = bench_parts
            if impl not in IMPLS:
                impl = "_"
                param = "/".join(bench_parts)
        else:
            continue  # unexpected shape

        try:
            with est.open() as f:
                ns = json.load(f)["mean"]["point_estimate"]
        except (KeyError, json.JSONDecodeError):
            continue

        out.setdefault(group, {}).setdefault(param, {})[impl] = ns

    return out


def render(data) -> str:
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
        # Decide if this group has any iced data — controls column shape.
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

            # Sweeten-only groups (e.g. reverse_50, justify_content_sweeten_only_20)
            # store their times under "_" instead of "sweeten" since the
            # bench id has no impl prefix. Treat them as the sweeten cell.
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
                lines.append(
                    f"| {param_disp} | {sw_s} | {ic_s} | {ratio} |"
                )
            else:
                lines.append(f"| {param_disp} | {sw_s} |")

        lines.append("")

    return "\n".join(lines)


if __name__ == "__main__":
    print(render(collect()))
