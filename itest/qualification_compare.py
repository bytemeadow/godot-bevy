#!/usr/bin/env python3
from __future__ import annotations


def compare_mutant_outcomes(
    baseline: dict[str, str], current: dict[str, str]
) -> tuple[list[str], list[str]]:
    regressions = sorted(
        stable_id
        for stable_id, outcome in baseline.items()
        if outcome == "caught" and current.get(stable_id) == "missed"
    )
    new_ids = sorted(set(current) - set(baseline))
    return regressions, new_ids
