# Scope — fixing cramped edge labels (Issues #2, #3, #4)

**Status:** Scoping. The three remaining gallery-quality issues —
Supervisor `creates`/`panics` cramping (#2), dependency-graph
crossings (#3), CI/CD `pass`/`skip` cramping (#4) — share related
roots but split into two distinct fixes. This document covers
both paths so the user can pick what to ship next.

## TL;DR

| Issue | Root | Fix path | Effort | Status |
|---|---|---|---|---|
| **#2 Supervisor** | parallel labelled edges between same node pair, plus narrow subgraph | Parallel-edge channel allocation in our own layout | 14-22h, 2-3 ship cycles | **Recommended next** |
| **#4 CI/CD** | parallel labelled edges (different styles) — same root as #2 | Same fix as #2; trivial extra cost once #2 lands | 0h additional | **Recommended next** |
| **#3 Dependency graph** | edge crossings in dense graphs; needs sugiyama improvements | (a) Adopt `ascii-dag` 0.9.1 for layered layout, OR (b) write Brandes-Köpf ourselves | 3-5 days for (a); 5-7 days for (b) | **Defer** to its own scope cycle |

**Recommended order:**
1. **0.12.0**: parallel-edge channel allocation (#2 + #4). Closes the
   most-cited gallery cramping. Honest 14-22h, 2-3 ship cycles.
2. **Then**: pause and reassess. If #3 is still bothering users (or if
   we want to continue feature work), evaluate `ascii-dag` adoption
   in a separate scope doc.

`ascii-dag` 0.9.1 was re-audited and looks genuinely viable for #3 —
clean two-stage API (`compute_layout() -> LayoutIR` with public
coordinate fields, dummy nodes as first-class, subgraph bounding
boxes, edge label positions, multi-segment waypoint routing). But
adopting it is a 3-5 day rewrite of `layered.rs` + `render/unicode.rs`
and not the right thing to mix with the parallel-edge fix.

---

## Issues #2 + #4: parallel-edge channel allocation

### What's happening today

Both issues come from the same structural gap: when N labelled
edges connect the same node pair, they share **one** inter-layer
column/row. The `label_gap` helper in `layered.rs:1026` sizes that
gap by `max(label_width)` plus a stacking row-count — but in LR
flow the labels still share the **same horizontal column**, so
visually they remain glued together.

The 0.9.5 label-vs-border guard prevents pixel corruption (no
border characters get overwritten). The 0.10.x crossing-min passes
reduce crossings. But neither widens the corridor that the labels
live in.

### What we already have (don't redo)

- `compute_spread_attaches` (`render/unicode.rs:988`) handles
  "many edges into one node's border" by spreading attachment
  points across the border's perpendicular axis. It does NOT
  detect parallel edges between the same node pair as a special
  group — they look like distinct edges to it.
- `ParallelInfo` doesn't exist yet. Phase 1 below adds it.
- Forward-then-back back-edges (Supervisor's F→W + W→F) are now
  handled by 0.11.2's InnerArea routing — back-edges go to the
  perimeter. With parallel-channel allocation we'll need to decide
  whether the back-edge stays on the perimeter or joins the
  channel group (see "Cross-cutting concerns" in Phase 2).

### Phased plan

#### Phase 1 — parallel-edge detection (no visible change, ~2h)

- Add `Graph::parallel_groups() -> Vec<Vec<usize>>` returning groups
  of edge indices that connect the same unordered node pair (so
  `F→W` and `W→F` are in the same group).
- Add `ParallelInfo { group_id, count, ordinal }` derived helper
  for downstream consumers.
- 5-6 unit tests covering: single edge, two same-direction,
  bidirectional, three-between-pair, unrelated edges, self-loop.
- **No snapshot changes** — Phase 1 has no consumer yet.

**Ship as 0.11.3** (or queue for 0.12.0). Pure addition. Risk: low.

After Phase 1, run a one-shot diagnostic over the gallery counting
parallel-edge groups. Decision point: if very few hits, drop Phase
2's per-channel allocation in favour of a simpler "bump label_gap
by `(count - 1) * label_height` when parallel detected" heuristic.

#### Phase 2 — channel allocation + path bending (~10-13h, the real fix)

- `label_gap` widens the inter-layer gap when a parallel-edge group
  crosses it (room for N labels in distinct rows/columns).
- Each parallel-group edge's path is bent (via the existing
  `route_via_waypoints` mechanism from 0.10.0) to its allocated lane.
- Each label sits in its own row (LR) or column (TD) — labels stack
  cleanly instead of overlapping.
- Snapshot impact: every flowchart with a parallel-edge group will
  change (~8-15 gallery diagrams). Each diff reviewed individually
  and labelled IMPROVEMENT / NEUTRAL_DOCUMENTED / REGRESSION.
- Behind a `LayoutConfig::parallel_channel_allocation: bool` feature
  flag for one minor-version cycle so a regression hotfix is one
  flag-flip away.

**Ship as 0.12.0**. Risk: high blast radius (snapshot triage),
medium implementation risk (A* doesn't take "preferred lane" hints
today; we use waypoints as the proven mechanism).

Mid-Phase decision point: after gap widening but before path-bending,
inspect Supervisor + CI/CD by eye. If widening alone gives clear
separation, **stop here and ship Phase 2a**.

#### Phase 3 — subgraph interior padding (DEFER unless needed)

- When a subgraph contains parallel-group edges between members,
  expand `SG_BORDER_PAD` locally so labels have ≥1 cell of breathing
  room from the subgraph border.
- Touches `compute_subgraph_bounds` recursion — risk of
  nested-subgraph regression (Issue 1 was in the same code).
- ~3-4h work.

**Recommendation: defer.** After Phase 2 ships, screenshot the
gallery. If Supervisor still looks cramped, schedule Phase 3 as
0.12.1.

### Cross-cutting concerns

- **Forward + back-edge pairs (Supervisor):** the F→W/W→F pair is
  one parallel group of 2. The 0.11.2 InnerArea perimeter routing
  exists to keep back-edges from fragmenting forward-edge channels
  in larger diagrams. Parallel-channel allocation should override
  perimeter routing **only when both endpoints are in the same
  parallel group AND in the same subgraph** — preserving 0.11.2's
  win for cross-subgraph back-edges.
- **Layer-direction interaction**: lanes are perpendicular to flow
  (rows for LR/RL, columns for TD/BT). Add lane allocation to both
  branches symmetrically.
- **`compute_spread_attaches` interaction**: a parallel pair F→W
  and W→F has different src/dst, so it doesn't trigger the existing
  spread (which keys on shared base cell). Phase 2's logic is
  additive. No double-count.

### Total effort estimate

| Phase | Work | Hours |
|---|---|---|
| Phase 1 | Detection helper + tests | 2 |
| Phase 2 | Gap widening + path lanes + label placement | 8-10 |
| Phase 2 snapshot review | 8-15 diagrams, individual judgement | 2-3 |
| Phase 3 (DEFER) | Subgraph padding | 3-4 |
| Buffer (per 0.10.0 precedent: estimates grow ~50%) | | 2-3 |
| **Phase 1 + 2 ship target** | | **14-18h** |

---

## Issue #3: edge crossings (separate, deeper)

### What's happening today

The dependency graph has multiple visible edge crossings.
App→PostgreSQL crosses Worker→PostgreSQL and detours around
Cache/RabbitMQ in layer 1. GitHub's Mermaid renders the same
graph cleanly because it does proper sugiyama-style layout
(crossing minimisation + long-edge dummy nodes + Brandes-Köpf
coordinate assignment).

### What we have

- Barycenter + median + transpose crossing-min (0.10.1).
- Long-edge waypoints (0.10.0) — gives long edges hint paths
  but doesn't fix the underlying crossings.
- No long-edge dummy nodes that ACTUALLY participate in
  barycenter (the 0.10.0 attempt was reverted because they
  produced uglier outputs without coordinate compaction).
- No Brandes-Köpf coordinate assignment.

### Two paths

#### Path A: adopt `ascii-dag` 0.9.1

Re-audited 2026-04-22. Verdict: genuinely viable.

- Public two-stage API: `compute_layout() -> LayoutIR`.
- `LayoutIR.nodes()` returns `LayoutNode { x, y, width, height,
  level, level_position, kind: Explicit/Implicit/Dummy }`.
- `LayoutIR.edges()` returns `LayoutEdge { from_x, from_y, to_x,
  to_y, path: EdgePath, label, label_position }`.
- `LayoutIR.subgraphs()` returns subgraph bounding boxes.
- Configurable `CrossingReducer` presets (FAST/STANDARD/QUALITY).
- Active maintenance, dual MIT/Apache-2.0, zero-dep `no_std`.

**Adoption replaces** `layered.rs` (1500+ LoC) and the layout call
in `render/unicode.rs`. We keep our box renderer, label placement,
ANSI color, subgraph border drawing, etc.

Effort: 3-5 days. Snapshot blast radius: massive (every flowchart
+ state diagram).

#### Path B: write Brandes-Köpf + dummy nodes ourselves

We already have the algorithm references. ~5-7 days.

Pros: full code ownership, no dep risk.
Cons: more work, harder to maintain, harder to correctly tune.

### Recommendation for #3

**Defer #3 entirely until after Phase 1+2 of the parallel-edge fix
ships.** Once we see how much the gallery improves after the
parallel-edge work, we can decide if the dependency-graph crossings
still bother us enough to justify the multi-day rewrite.

If we DO commit to #3 later, prefer Path A (`ascii-dag`) over Path
B unless the snapshot triage reveals deal-breakers.

---

## Open questions for the user

1. **Approve Phases 1+2** of the parallel-edge work as the next ship
   (target: 0.12.0)?
2. **Phase 3 (subgraph padding)**: schedule now (as 0.12.1), or wait
   for the Phase 2 result before deciding?
3. **#3 deferral**: agreed to defer until after parallel-edge fix
   lands?
4. **Phase 1 alone** can land NOW as a no-op-pure-addition release
   (0.11.3) — preference for that vs holding back until Phase 2 is
   ready?

Once approved, I'll create the per-phase task list and start
building.
