# Changelog

All notable changes to `markdown-tui-explorer` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.25.0] - 2026-04-24

### Internal — centered-popup layout helpers consolidated (Ship 2 follow-up B)

`centered_rect` / `centered_pct` / `percent_rect` were duplicated
across 8 popup files (`help`, `link_picker`, `config_popup`,
`copy_menu`, `tab_picker`, `mermaid_modal`, `table_modal`,
`search_modal`). Consolidated into `src/ui/layout.rs` — one source
of truth for centered-popup math.

Three helpers, two with shared shape + one that diverges:

- **`centered_rect(width, height, area)`** — fixed cells, used by
  5 files (help, link_picker, config_popup, copy_menu, tab_picker).
  Byte-for-byte identical across all 5 — pure consolidation.
- **`centered_pct(w_pct, h_pct, area)`** — percentage with floor
  10×5, used by 2 files (mermaid_modal, table_modal). Identical.
- **`percent_rect(w_pct, h_pct, area)`** — percentage with floor
  20×4, used by search_modal only. Kept separate from
  `centered_pct` because the search modal needs more vertical
  space at small terminal sizes (load-bearing difference).

Net: ~65 lines of duplicated definitions deleted, 55 lines added
to one place.

Module location: `src/ui/layout.rs` (not `src/theme/layout.rs`)
because these are rect-math primitives, not theme tokens.
`theme::Spacing` answers "how many cells of padding"; `centered_rect`
answers "where does this popup go" — different layers.

### Internal — render_code_block migrated to Tokens (Ship 2 follow-up C)

First validation of the Ship 2 token-migration story. The renderer
now derives `Tokens` from the active theme alongside the existing
`Palette` reference, and `render_code_block` reads its three
relevant fields from semantic slots:

| Was (Palette) | Now (Tokens) |
|---|---|
| `palette.code_border` | `tokens.syntax.code_border` |
| `palette.code_fg` | `tokens.syntax.code_fg` |
| `palette.code_bg` | `tokens.surface.raised` ← non-obvious sourcing now visible |

The `surface.raised` rename is the standout win: a reviewer reading
`palette.code_bg` had no way to know that code blocks, popups, and
the status bar all share the same raised-surface tier — that
sourcing decision lived inside `From<Tokens> for Palette`. The
new name surfaces it at every call site.

Public API unchanged. Per-color cached struct fields stay in place
for now because `DisplayMath` and other render paths still read them;
finishing that migration is a follow-up.

842 tests pass, clippy + fmt clean.

## [1.24.1] - 2026-04-24

### Internal — spacing migration audit (Ship 2 follow-up A)

Audited every remaining `Constraint::Length(N)` site in `src/ui/`
against the `theme::Spacing` scale to confirm whether more
literal-N → `Spacing::*` migrations were possible. Result: **none.**
Every remaining literal is a runtime variable (centered-popup
heights/widths sized to fit content, tab-bar heights of 0/1
depending on tab presence, gutter widths scaling with line-count).
The five sites migrated in 1.24.0 were the complete scale-relevant
set.

Three content-policy sites (`help.rs`, `copy_menu.rs`,
`config_popup.rs`) gained a brief `// content-sized: …` comment
documenting *why* the literal is correct as a literal, so a future
contributor doesn't try to migrate them by mistake. Plus a
`cargo fmt` pass that fixed pre-existing style violations
introduced during Ship 2.

Confirmed observation worth following up: the `centered_rect` /
`centered_pct` / `percent_rect` helpers are duplicated across 8
files with the same shape. ROADMAP already tracks this as a
`theme::layout::centered_rect` consolidation.

## [1.24.0] - 2026-04-24

### Added — design tokens, ColorOps, Spacing scale (Ship 2)

Refactor of the theme system to introduce a semantic-token layer.
Behavior-preserving — every existing caller continues to read
`palette.foreground` etc. unchanged.

**`theme::Tokens`** is the new source of truth for each theme. Tokens
are nested into per-purpose sub-structs (`Surface`, `Text`, `State`,
`Accent`, `Syntax`, `Heading`, `Status`, `List`, `Table`, `Git`) so
`tokens.state.selection_bg` reads as "selection state, background"
rather than "row 8 of a 33-field flat struct". Each theme is now a
small `fn` in `theme::themes` listing its base hues + assignments.

**`theme::Palette`** is now auto-generated from `Tokens` via
`From<Tokens>`. The existing 33-field flat shape stays — every caller
already on `palette.<field>` continues to compile and behave
identically. The `From` impl is the migration boundary: forgetting a
field there fails to compile, so no Palette slot can ever be silently
uninitialised.

**`theme::ColorOps`** trait adds `lighten`/`darken`/`is_light` for
linear-RGB blending toward white/black. Hand-rolled in <30 lines (no
`palette` crate dep). Today's themes don't derive — every theme
ships a designer-chosen palette. Available for future themes,
user-customizable themes, or per-theme tweaks where derivation is
cleaner than a literal.

**`theme::Spacing`** enum (`Xs` `Sm` `Md` `Lg` `Xl` → `1 2 3 5 8`
cells, Fibonacci-ish jump set) with `From<Spacing> for Constraint`.
Five sites migrated as proof-of-use: outer status bar (`mod.rs`),
editor footer (`editor.rs`), search modal query/footer
(`search_modal.rs`). Remaining ~15 sites deferred to a follow-up.

### Added — new invariant tests

Two new theme-invariant tests run over all 8 themes:

- **`selection_bg_is_distinct_from_surfaces`** — derived selection
  background must be at least 3 luminance units (scaled 0-100) away
  from `surface.base` and `surface.raised`. Catches the original
  Solarized Light bug *automatically*: even if a future contributor
  hard-codes `selection_bg = surface.raised`, the test fires before
  the change ships.
- **`focus_is_visible_against_surface`** — focus ring colour must
  meet WCAG SC 1.4.11 (3:1 vs `surface.base`) — relaxed from text
  AA because focus is a decoration line, not text.

Plus 5 ColorOps unit tests (round-trip, identity at f=0, endpoints
at f=1, named-color passthrough, 50% midpoint) and 2 Spacing tests
(monotonic, `Into<Constraint>` correctness).

### Changed — Nord theme palette label/slot fixes

The Polar Night gradient labels were swapped from canonical (carryover
from 1.23.0): what was called "nord1" was actually canonical nord2,
"nord3" was canonical nord1, etc. Fixed labels to match
[Nord's official palette](https://www.nordtheme.com/docs/colors-and-palettes).
Selection background lifted from nord2 → nord3 (the most-elevated
Polar Night tier) to clear the new `selection_bg distinct from
surfaces` invariant — adjacent tiers measured Δ=1.7, perceptually
too close.

### Internal — file layout

`src/theme.rs` (~700 lines) split into a `src/theme/` directory:
`mod.rs` (Palette + Theme + From<Tokens>), `tokens.rs` (token types
+ derivation invariants), `themes.rs` (per-theme constructors),
`contrast.rs` (WCAG + ColorOps), `spacing.rs`. The split was
optional but justified — `themes.rs` is the file a contributor
visits when adding a new theme; it should not be buried inside the
type-definition file.

842 tests pass, clippy + fmt clean.

## [1.23.0] - 2026-04-24

### Added — theme contrast / palette-invariant audit (Ship 1)

New unit tests parameterised over all 8 themes catch two classes of
defects automatically; CI fails if any new theme (or palette tweak)
re-introduces them:

- **Highlight invisibility.** `selection_bg` and `current_match_bg`
  must differ from both `code_bg` and `background`. Three themes
  shipped with `selection_bg == code_bg` — cursor highlight inside
  code blocks was literally invisible (the 2026-04-24
  solarized_light report).
- **WCAG AA reading-text contrast (≥ 4.5:1).** Reading-text fg/bg
  pairs (foreground/background, code, selection, on-accent,
  search/current match, status bar) must meet AA. Decoration
  (borders, gutters) is excluded — thin strokes tolerate lower
  contrast and pinning them inflated rejections without visible
  win. Named colours (terminal-defined RGB) skip the check
  silently — only `Color::Rgb(...)` slots are asserted.

### Fixed — palette adjustments (visible win, no API change)

19 contrast misses + 3 selection collisions across 6 themes.
Adjustments stay within each theme's canonical palette where
possible (e.g. Solarized base01/base02, Gruvbox bg1/bg2/fg) and
fall back to true black/white only when the canonical palette
can't reach AA (typically text on a vivid accent colour).

- **Dracula:** `on_accent_fg` 248,248,242 → 40,42,54 (was 2.26:1
  on purple); `status_bar_fg` comment → foreground (was 3.03:1).
- **Solarized Dark:** `selection_bg` base02 → base01 (was identical
  to `code_bg`); `selection_fg` base1 → base3; `code_fg` base0 →
  base1 (was 4.11:1); `on_accent_fg` base1 → black (was 1.38:1
  on blue); `match_fg` base03 → black (was 3.26:1 on orange);
  `status_bar_fg` base01 → base1 (was 2.42:1).
- **Solarized Light:** `selection_bg` base2 → base1 (was identical
  to `code_bg`); `foreground` and `code_fg` base00 → base02 (were
  4.13:1 / 3.64:1 — Solarized's intentional "soft" contrast loses
  to AA in a TUI markdown reader); `on_accent_fg` base3 → black
  (was 3.41:1 on blue); `match_fg` base3 → black (was 2.98:1 on
  yellow); `status_bar_fg` base00 → base02; `selection_fg`
  base01 → base02.
- **Nord:** `match_fg` nord0 → black (was 3.05:1 on red);
  `status_bar_fg` nord2 → nord4 (was 1.36:1 — basically illegible).
- **Gruvbox Dark:** `match_fg` 40,40,40 → black (was 4.29:1, just
  under); `status_bar_fg` gray → fg (was 3.58:1).
- **Gruvbox Light:** `selection_bg` bg1 → bg2 (was identical to
  `code_bg`); `match_fg` bg → black (yellow 2.19:1, orange 3.41:1).

Solarized purists may note the foreground/code text is now darker
than spec — the original base00 ships sub-AA by design ("soft
reading"). For a TUI markdown reader, AA wins.

## [1.22.5] - 2026-04-24

### Changed

- **Bumped `mermaid-rs-renderer` from 0.2.1 → 0.2.2.** Picks up the
  state-diagram-v2 fix we reported as
  [1jehuang/mermaid-rs-renderer#67](https://github.com/1jehuang/mermaid-rs-renderer/issues/67)
  ("missing state names and clipped transition labels"). Image-mode
  state diagrams now keep state titles and accumulate descriptions
  correctly. Bonus fixes inherited from the upstream release:
  sequence-diagram `alt` frame geometry (no more layout panic on
  wide section labels), compact-flowchart label decorations, dotted
  edges visually distinct from solid, class diagram stereotypes
  rendered above (not as) members, class arrowheads no longer hidden
  under node boxes, empty-subgraph layout panic, non-ASCII hex color
  panics, and a new compact Gantt display mode.

## [1.22.4] - 2026-04-24

### Fixed — code-block ASCII art alignment

Pre-formatted multi-row text inside code blocks (e.g. the `┌──┐` /
`│ Build │` / `└──┘` chart in the README) was rendering with
top/bottom borders misaligned with the text row. Root cause: the
text-wrapping pass split each line at whitespace and rejoined words
with single spaces — fine for prose, wrong for ASCII art whose
multi-space gaps between adjacent boxes are load-bearing for
alignment. The middle row `│ Build │─────▸│ Test │─────▸│ Deploy │`
has no internal whitespace so its widths were preserved, but the
top row `┌───────┐      ┌──────┐      ┌────────┐` collapsed to
`┌───────┐ ┌──────┐ ┌────────┐`, leaving the second/third boxes
with their borders shifted left of their walls. Visual result: only
the first box (Build) appeared to have a proper outline; subsequent
boxes (Test, Deploy) looked like text-with-side-walls.

Fix: short-circuit `emit_wrapped_hard_line` when the input fits in
`max_width` — emit verbatim, no whitespace splitting. Word-splitting
still runs when the line would actually overflow.

## [1.22.3] - 2026-04-24

### Fixed — mermaid-text 0.16.3 source-attach (final form)

The 1.22.2 perpendicular-axis heuristic still over-anchored LR
layouts with vertical first steps (back-edges, mid-side attach
points in LR-with-internal-TB subgraphs). The new rule applies the
anchor only to TD/BT layouts whose route turns sideways at the
source — the only case where the cell would otherwise render as a
stub `─` adjacent to a horizontal box border. Supervisor-style
charts now render `││` cleanly (was `│┐`/`│┘` in 1.22.1 and 1.22.2).

### Docs

- **README static text examples regenerated.** The "Unicode" version
  of the Build/Test/Deploy ASCII-fallback example used `+---+`
  ASCII-style corners (a copy-paste leftover from the ASCII variant
  beneath it). Now uses proper `┌───┐`. The Sugiyama-backend
  dependency-graph example had stray vertical lines below the
  diagram from a stale render; regenerated against current code.

## [1.22.2] - 2026-04-22

### Fixed — mermaid-text 0.16.2 source-attach correction

The 1.22.1 release applied the source-attach anchor unconditionally,
which produced spurious corner glyphs (`┐ ┘ ┌ └`) on edges whose
first step already ran in the layout's natural flow direction —
breaking back-edges, multi-edge fan-outs, and LR layouts containing
internal TB subgraphs (Supervisor pattern). The 1.22.2 release
applies the anchor only when the route's first step is
*perpendicular* to the natural axis, restoring clean `│`/`─` for
parallel starts while keeping the corner anchor for true L-turns
out of source boxes. L-route bend now also prefers the target side
on cost ties, reducing crossings on dense graphs.

## [1.22.1] - 2026-04-23

### Fixed — mermaid-text 0.16.1 polish from real-doc testing

Reported on flowcharts and a sequence diagram in a user's
`personal_notes.md`:

- **Edge labels now honour `<br>`** the same way node labels do —
  `|"recommendations.getFeed,<br/>records event"|` no longer
  renders the literal `<br/>` inline. Surrounding quotes stripped
  too.
- **Sequence participant labels and message text** strip `<br>`
  to a single space (renderer is single-row in those positions —
  `\n` would break the layout). Notes still convert to `\n` since
  they have multi-line box support.
- **Edges crossing subgraph borders** show a proper junction glyph
  (`┴ ┬ ├ ┤ ┼`) at the crossing instead of the bare border line.
  Previously the route's vertical/horizontal segment was hidden by
  the protected border, making edges look "missing their initial
  portion" through subgraph boundaries.
- **Edge attach points anchor visually to the source box border**
  via a corner glyph (`└ ┘ ┐ ┌`). An edge whose source/target
  columns differ by one (boxes of different widths — common when
  layout pins boxes to their content) no longer looks detached at
  the source side.

## [1.22.0] - 2026-04-23

### Added — Phase 5 of the architecture cleanup: classDiagram support

Closes the largest "0% coverage" Mermaid diagram-type gap. UML class
diagrams (the third-most-used Mermaid type after flowchart and
sequence; staple of architecture/UML docs) now render in the viewer.

User-visible: paste a `classDiagram` block into any markdown file and
it renders alongside the existing flowchart / state / sequence / pie /
ER support. All 7 UML relationship types render with their proper
endpoints (`△` inheritance/realization, `◆` composition, `◇`
aggregation, arrows for association/dependency). ASCII fallback maps
each glyph to a distinct character (`^ # *`).

Internal — see `crates/mermaid-text/CHANGELOG.md` (mermaid-text 0.16.0)
for the full change list:
- New `class.rs` data model + `parser/class.rs` parser (37 unit tests).
- New `render/class.rs` renderer that synthesises a layered Graph for
  positioning and uses Phase 4's A\* router for edge routing.
- Extracted `render/box_table.rs` from `render/er.rs` — both renderers
  now share the box-with-attribute-table primitive (~150 LOC reduction
  in ER + zero duplication).
- 6 new snapshot fixtures + width-sweep + fuzz harness (50 mangled
  inputs, fixed-seed) guaranteeing parser never panics.

Tests: 545 mermaid-text tests pass (was 472); 284 binary tests pass
(unchanged). Clippy + fmt clean.

This phase ships the **5-phase architecture cleanup** in full:
1. text_layout foundation (1.20.4)
2. wrapped-cell tables (1.20.5)
3. own prose wrapping; visual_rows.rs deleted (1.21.0)
4. mermaid-text A\* edge routing (1.21.1)
5. classDiagram support (1.22.0)

## [1.21.1] - 2026-04-23

### Changed — Phase 4 of the architecture cleanup

- **mermaid-text 0.15.0**: edge routing consolidated into a single A\*
  pass per edge with try-straight → try-L fast path. Direction-aware
  crossing costs (`EdgeOccupiedHorizontal` / `EdgeOccupiedVertical`)
  let A\* avoid ugly overlaps while accepting clean perpendicular
  crossings. ~450 LOC of waypoint-hinting machinery deleted from the
  layered backend; per-edge dispatch consolidated into a new
  `layout::router` module. 19 new crossing-counter regression tests +
  5 dense-graph fixtures guard against tuning drift. See
  `crates/mermaid-text/CHANGELOG.md` for the full deletion list.

User-visible: flowcharts route more cleanly on average — fewer
zigzags through unrelated nodes, edge crossings prefer perpendicular
junctions over same-axis overlaps. All 63 existing visual snapshots
either match or have been reviewed and accepted as improvements.

## [1.21.0] - 2026-04-23

### Changed — Phase 3 of the architecture cleanup: own prose wrapping; visual_rows.rs deleted

The viewer no longer delegates wrapping to ratatui's `Paragraph::wrap`.
`DocBlock::Text` now carries a stable `TextBlockId`; the viewer caches a
`WrappedTextLayout { wrapped, physical_to_logical }` per block,
populated whenever `layout_width` changes — exactly the pattern Phase 2
established for tables. `block.height()` reads from the cache.

The visual-vs-logical rift introduced in 1.18.4 (and patched
reactively in 1.18.5) collapses back into one coordinate space:
`cursor_line`, `scroll_offset`, `total_lines`, link/anchor positions,
and search match positions all agree, by construction.

Internal:
- `src/ui/markdown_view/visual_rows.rs` — **deleted**.
- `update_text_visual_heights` → `update_text_layouts`. Populates the
  cache and updates `wrapped_height`.
- `source_line_at_width` → `source_line_at`; `logical_line_at_source_width`
  → `logical_line_at_source`. Both now consume the layout caches
  (`text_layouts` + `table_layouts`) instead of recomputing wrap on
  every call.
- `current_line_width` is 5 lines, reads cached `WrappedLine.width`.
- `apply_visual_or_cursor_highlight` lost the visual-→-logical
  conversion; cursor index = `cursor_line - block_start` directly.
- Text blocks render via plain `Paragraph::new(text).scroll((skip, 0))`;
  `Wrap { trim: false }` is gone.
- `WrappedLine::to_ratatui_line()` re-introduced as the single
  conversion site (previously hand-rolled in three places).
- `gutter.rs` extracted `build_gutter_lines` so the line-number logic
  is unit-testable without a `Frame` (5 new direct tests).
- `collect_match_lines` Text branch consults the cache; visual row =
  match index.
- Char-mode visual yank iterates the cached wrapped rows. Previously
  it iterated `text.lines` (logical) treating indices as visual rows
  — broken for any wrapped paragraph.

User-visible: nothing should change. Cursor, scroll, gutter, links,
search, yank all behave the same way they did in 1.20.5; the
implementation is just architecturally honest.

Tests: 284 binary tests pass (was 267 before Phase 3 work — +17 net,
including 12 new Phase 3 cases and 5 new gutter unit tests). 351
mermaid-text tests pass. Clippy + fmt clean.

Audit gate: Explore-agent pass found 1 real ship-blocker (char-mode
yank used logical line indices as visual rows — fixed before this
commit), 1 clarity nit on the gutter increment logic (refactored to a
single advance per emit), 1 stale doc comment (corrected). The plan's
"Phase 3.5" follow-up: merge `apply_block_highlight` and
`apply_visual_or_cursor_highlight` once the table path's clip-start
offset semantics are unified with text's full-block view.

## [1.20.5] - 2026-04-23

### Changed — Phase 2 of the architecture cleanup: wrapped-cell tables

Wide table cells now **wrap into extra physical rows** instead of
truncating with an ellipsis. Closes the largest user-visible markdown
gap surfaced by the research note (Suggestion 3). Both the inline
viewer and the expanded modal switch in this single ship.

User-visible:
- A 200-character cell on a narrow terminal renders across multiple
  rows with full content preserved, instead of `…`-truncated to fit.
- Vertical bars stay column-aligned across every physical sub-row of a
  given markdown row (top-aligned shorter cells; padded with blanks on
  continuation rows).
- The `[press ⏎ to expand]` hint disappears from inline tables that
  previously truncated — there's nothing to expand to anymore, the
  modal renders the same wrapped output.
- Header/body separator (`├─┼─┤`) fires only after the *last* sub-row
  of the header. No inter-body separators (matches GitHub / pandoc /
  termimad convention).

Internal:
- New private `WrappedRow` + `wrap_table_rows` + `emit_row_lines` in
  `src/ui/table_render.rs`. The expanded modal calls the same helpers
  — modal and inline are one pipeline.
- `state::TableLayout` gains `physical_to_source: Vec<u32>` so
  jump-to-source still lands on the right markdown row when the cursor
  sits on a wrapped sub-row. `source_line_at_width` takes the cache as
  a 4th argument; pre-draw fallback math preserves no-wrap behavior.
- `layout_table` returns `(Text, height, Vec<u32>)` instead of
  `(Text, height, bool)` — `was_truncated` is gone because nothing
  truncates any more.

Deletions (per the plan's "no dead surface area" gate):
- `src/ui/table_modal.rs::wrap_cell_spans` and its private helpers
  (`emit_wrapped_hard_line`, `merge_char_style_pairs`, `StyledChar`)
  + 7 unit tests — superseded by `text_layout::wrap_spans`.
- `src/markdown/mod.rs::cell_display_width` — superseded by
  `text_layout::measure`. Two callers in `markdown/renderer.rs`
  migrated.
- `src/ui/table_render.rs::truncate_spans` + 2 unit tests — wrapping
  replaces truncation.
- `was_truncated` flag in `layout_table` return tuple.

Tests:
- 5 new unit tests in `table_render.rs` covering wrap width-sweep,
  mixed-height row alignment, header-separator placement, no
  inter-body separators, `physical_to_source` mapping.
- 5 new snapshot tests via `insta` (added as a dev-dependency)
  covering 2-col / 5-col / styled / modal rendering.
- 11 deleted (the `wrap_spans_*` and `truncate_spans_*` tests of the
  retired helpers).
- 267 binary + 351 mermaid-text tests pass; clippy + fmt clean.

Net source-line delta: roughly **-180 lines**. Phase 2 is a deletion
phase with a feature on top.

## [1.20.4] - 2026-04-23

### Internal — Phase 1 of the architecture cleanup

Foundational refactor with no user-visible behaviour change. First step
of the 5-phase plan in `docs/markdown-text-architecture-plan.md`.

- New module `src/text_layout.rs` — single source of truth for
  display-width calculation over ratatui span lists.
  - `WrappedSpan { content: String, style, width: u16 }` — owned styled
    chunk with cached display width.
  - `WrappedLine { spans, width }` — one wrapped visual row.
  - `wrap_spans(spans, max_width) -> Vec<WrappedLine>` — greedy
    word-wrap; algorithm ported verbatim from
    `table_modal::wrap_cell_spans` so a Phase 2 swap is mechanical.
  - `measure(spans) -> u16` — total display width without allocation.
- `visual_rows::line_visual_rows` is now a 4-line adapter over
  `wrap_spans`. The old hand-written ceil-div on `UnicodeWidthStr`
  is gone; layout-width math has one implementation.
- `state::current_line_width` and `highlight::apply_block_highlight`
  use `text_layout::measure` instead of inline span-width sums.

Tests: +14 cases in `text_layout::tests`, including a width-sweep
harness over `[20, 40, 60, 80, 120, 200]`, idempotence (soft-wrap
inputs only — explicitly documented), hard-newline consumption,
combining-mark glue, wide CJK, mixed styles across wrap boundaries,
and `max_width == 0` short-circuit. 266 binary tests + 351 mermaid-text
tests still pass; clippy + fmt clean.

Quality gates audited (per `docs/markdown-text-architecture-plan.md`):
no dead code, no `#[allow(dead_code)]`, no unused dependencies, no
duplicated width-sum loops anywhere outside `text_layout::measure`,
rustdoc on every `pub` item.

Phases 2 + 3 (wrapped-cell tables, deletion of `visual_rows.rs` once
prose owns its wrapping) build directly on this module.

## [1.20.3] - 2026-04-23

### Changed

- **mermaid-text 0.14.5**: layered backend's barycenter sweep now
  augments the edge list with dummy nodes for long forward edges
  (one per intermediate rank). Dagre / graph-easy both do this so
  the within-layer ordering step sees a uniform graph; without it,
  long edges only nudge their endpoints during sorting and
  intermediate-layer real nodes stay where they happened to land.
  Visible win on flowcharts where a "skip" edge spans multiple
  layers occupied by other real nodes.

  First step of a planned layout-quality pass — next candidates
  (per a survey of dagre + graph-easy patterns): A* edge routing
  with crossing/turn penalties (graph-easy `Scout.pm`) and
  Brandes-Köpf x-coordinate assignment (dagre `position/bk.ts`).

## [1.20.2] - 2026-04-22

### Added

- **Request the Kitty keyboard enhancement protocol on startup.**
  Modern terminals (Ghostty, Kitty, WezTerm, recent iTerm2, foot)
  honour `PushKeyboardEnhancementFlags` and start sending precise
  modifier flags — Cmd surfaces as `KeyModifiers::SUPER`,
  distinguishable from `ALT` (Option / Esc-prefixed sequences).
  Without it, Cmd+arrow and Option+arrow both arrived as
  ALT-modified to the legacy keyboard layer, so the viewer couldn't
  bind them to different actions.

  Concrete win for Ghostty users with `macos-option-as-alt = true`:
  Cmd+Left/Right now triggers the line-start/end binding (via
  `SUPER+arrow`, added in 1.20.1) while Option+Left/Right keeps
  doing word jumps. macOS-native cursor behaviour out of the box.

  Older terminals (Terminal.app, Alacritty) silently ignore the
  request and keep working with the legacy fallbacks (Esc+f / Esc+b
  / Alt+arrow CSI codes — all still wired).

  Pop the flags on shutdown via `TerminalGuard::drop` so the
  terminal returns to its default mode after the app exits.

## [1.20.1] - 2026-04-22

### Fixed

- **Option+Right no longer pops the link picker.** macOS terminals
  (Terminal.app, iTerm2 default) send Option+Right as the literal
  bytes `Esc f` (the readline word-forward chord), which crossterm
  decodes as `KeyCode::Char('f')` with `KeyModifiers::ALT`. The bare
  `f` arm — which opens the `f` link picker — caught the Alt-modified
  variant too. Added explicit `Alt+f` / `Alt+b` arms ahead of the
  bare ones so word-jumps fire instead.

### Added

- **Cmd+Left/Right line jumps via Kitty keyboard protocol.**
  Crossterm reports Cmd as `KeyModifiers::SUPER` on terminals that
  speak the Kitty enhancement protocol (Kitty, recent WezTerm,
  iTerm2 with the protocol enabled). Bound `SUPER+Left/Right` to
  line start / end so users on those terminals get native macOS
  Cmd+arrow behaviour. On terminals that don't speak the protocol,
  Cmd+arrow either gets intercepted by the OS (no-op in the app)
  or arrives as Home/End / Esc+arrow — both already wired.

## [1.20.0] - 2026-04-22

### Added

- **Word-jump cursor keys.** The viewer's horizontal cursor now
  honours macOS-standard chords plus vim word motions:
  - **Option+Left / Option+Right** (Alt+Left/Right on Linux) — jump
    by whitespace-separated word.
  - **Home / End** (Cmd+Left / Cmd+Right via Terminal.app forwarding)
    — jump to line start / end.
  - **`w`** — next word; **`b`** — previous word; **`e`** — same as
    `w` for now (the viewer has no "yank to end of word" so the two
    semantics collapse).
  - **`^`** — line start; **`$`** — line end.

  Visual mode (`v`) extends the selection through word jumps too, so
  Option+Right after `v` selects a word at a time.

  Word definition is the simple "maximal run of non-whitespace"
  segmentation — same as terminal `readline` and most editors'
  default. Indexed by char position; multi-byte / wide chars (CJK,
  emoji) get the same approximation as the existing single-cell
  `h`/`l` arrows.

  Covered by 7 unit tests on `next_word_col` / `prev_word_col`.

## [1.19.2] - 2026-04-22

### Fixed

- **Mermaid modal text-zoom now responds to every press.** 1.19.1
  used `max_width`-based compaction, but mermaid-text only has three
  discrete compaction levels and only triggers them when budget <
  natural width — so once the diagram fit the budget, further
  presses did nothing (the user reported `+` worked once then `-`
  reset and that was it).

  Switched to driving the renderer with explicit `(layer_gap,
  node_gap)` overrides instead of a width budget. Defaults are
  `(6, 2)`; each `+` step adds `+2`/`+1`, each `-` step subtracts
  `2`/`1`, clamped to `[0, 24]` and `[0, 10]`. Result: every press
  produces a deterministically different layout (until the clamp
  hits), so the diagram visibly grows or shrinks as you'd expect.

  Required a new `gaps_override: Option<(usize, usize)>` field on
  `mermaid_text::RenderOptions` (mermaid-text 0.14.4) and a new
  `crate::mermaid::try_text_render_with_gaps` helper.

  Sequence diagrams still ignore zoom (no compaction pipeline at
  all). Pie / erDiagram ignore the gap override too — they have
  their own layout pipelines and respond only to `max_width`.

### Changed

- **mermaid-text 0.14.4**: add `RenderOptions::gaps_override` to
  expose `(layer_gap, node_gap)` directly, bypassing the
  `max_width`-driven compaction pipeline. Existing callers see no
  behaviour change (default `None`).

## [1.19.1] - 2026-04-22

### Fixed

- **Mermaid modal text-zoom now actually changes the diagram.** 1.19.0
  shifted the budget by ±20 cols per press, but `mermaid-text` only
  triggers compaction when the budget is *below* the natural rendering
  AND it returns the first compact level that fits — so a 20-col delta
  rarely crossed a threshold and the user only saw the footer change,
  not the diagram itself.

  The new formula:
  - `+` → request **natural** size (`max_width = None`, no compaction).
  - `-` → multiplicative shrink, budget = `modal_width × 0.7^|zoom|`.
    Each press shaves ~30% off the budget so the renderer reliably
    walks down its three discrete compaction levels.
  - `=` → reset to `0` (budget = modal width).

  Caveat unchanged: sequence diagrams have no compaction pass at all
  (fixed layout), so zoom is a no-op there. Pie / erDiagram honour the
  budget directly. Flowchart / state run through the three-level
  compaction pipeline.

## [1.19.0] - 2026-04-22

### Added

- **Zoom keys for the text-mode mermaid modal.** When the chart is too
  big for the modal, press `+` to request a more spacious layout, `-`
  for a more compact one, and `=` to reset. Each press re-runs
  `mermaid-text` synchronously at an adjusted `max_width` budget
  (modal_width + zoom × 20 cols), so the new layout shows up
  immediately. Scroll position resets on each zoom step so you land at
  the top-left of the re-rendered diagram.

  Caveat: `mermaid-text` compacts in discrete steps (its
  `LayoutConfig` levels), so a single press may or may not visibly
  change the diagram depending on whether it crosses a threshold.
  Sequence/pie/erDiagram have a fixed minimum spacing and won't
  compact past it. The footer shows the current zoom level when
  non-zero.

  Image-mode entries ignore the zoom keys — the protocol already
  auto-fits bitmaps to the modal rect.

## [1.18.5] - 2026-04-22

### Fixed

- **Horizontal cursor arrows stopped working after scrolling into a
  wrapped paragraph.** Regression introduced by 1.18.4's switch to
  visual-row coordinates. `current_line_width()` still indexed
  `text.lines` by the visual-row offset; on a wrapped line that
  offset pointed past the end of `text.lines`, so width returned 0.
  Two downstream effects:
  1. `clamp_cursor_col()` (called after every `j`/`k`) then reset
     `cursor_col` to 0.
  2. The Right-arrow handler's upper bound became `max = 0`, so
     pressing `l` / Right was a no-op.

  Fix: convert the visual-row offset to a logical line index via
  `visual_row_to_logical_in_block` before looking up `text.lines`.
  Covered by a new `current_line_width_handles_wrapped_lines` test.

## [1.18.4] - 2026-04-22

### Fixed

- **Scroll math is now in visual rows, not logical lines.** 1.18.3 fixed
  the scroll-time line reveal for soft-broken paragraphs but the bug
  survived for single source lines that were themselves wider than the
  viewport — exactly what happens in documents with prose paragraphs
  written as one long physical line (common in note-taking tools).
  User reproduction: a 180-char line at source line 105 in
  `personal_notes.md` wrapped visually but `block.height()` still
  returned 1, so scrolling past it shifted the following table by the
  missing rows.

  The fix moves the entire coordinate system to visual rows:

  - `DocBlock::height()` now returns wrapped visual-row counts for
    `Text` blocks (via a new `visual_height: Cell<u32>`), recomputed on
    every layout-width change by `update_text_visual_heights`.
  - `scroll_offset`, `cursor_line`, `total_lines`, and the visual
    selection range are all in visual rows. `j` / `k` move by one
    visual row, matching pager conventions (`less`, `bat`) rather than
    strict vim logical-line semantics.
  - Text blocks render via `Paragraph::new(full_text).scroll((N, 0))`
    instead of slicing by logical line, so ratatui's wrap and our
    scroll math agree on what's visible.
  - `recompute_positions` translates logical-in-block link and anchor
    indices to absolute visual rows so the `f` link picker and TOC
    jumps still land on the right row under wrapping.
  - `collect_match_lines` records matches in visual rows so `n` / `N`
    doc-search navigation jumps don't drift when wrapped paragraphs
    sit between matches.
  - `source_line_at` and `logical_line_at_source` gain width-aware
    variants (`_width`) used everywhere that converts between cursor
    position and source-line number (edit mode entry, `yy` / visual
    yank, link-picker line filtering).

  Gutter line numbers now track logical source lines (with blank
  continuation rows) rather than absolute visual rows, so long
  paragraphs show a single number on the first wrap row and blanks
  below — the correspondence users expect from an editor/pager.

## [1.18.3] - 2026-04-21

### Fixed

- **Lines near tables no longer "shift" or "appear" while scrolling.**
  Reported on a long-prose-followed-by-table layout: scrolling past the
  paragraph would reveal a line of text or a blank that wasn't visible
  a moment earlier, and the table itself would shift up/down by one or
  two rows.

  Root cause: pulldown-cmark joined every soft break inside a paragraph
  into a single `ratatui::Line` (with a space between the joined parts).
  When that single Line was wider than the viewport, `Paragraph::wrap`
  expanded it to N visual rows, but the scroll math counted it as 1
  logical line. The mismatch left N-1 visual rows worth of content
  hiding behind the wrap overflow, only to "reveal" themselves once
  scrolling shifted the line out of the rendered rect.

  Fix: preserve source line breaks during rendering so each source
  line becomes its own `ratatui::Line` and the logical/visual line
  counts match for the common prose case. Soft breaks inside links,
  table cells, and list items still emit a space because those
  contexts can't represent a per-line split correctly (LinkInfo
  records a single line/col range; table cells render via the table
  layout; list items track their bullet/indent only at `Tag::Item`).

  Also: stopped restamping `current_source_line` on `Event::End`,
  which previously rewound source-line tracking to the start of a
  multi-line paragraph and put the trailing rendered line on the
  wrong source line. The two changes ship together because the soft
  break flush surfaced the latent stamping bug.

## [1.18.2] - 2026-04-22

### Added

- **Nix flake**. `flake.nix` at the repo root makes
  `nix run github:leboiko/markdown-reader` work out of the box,
  same for `nix profile install` and embedding as a flake input
  in another configuration. Closes the Nix distribution gap from
  the md-tui competitive analysis.

  Build is via `pkgs.rustPlatform.buildRustPackage` with the
  workspace `Cargo.lock` for reproducibility — Nix prefetches
  every crate before the sandboxed build, no network in
  `cargo build`. `cargoBuildFlags = [ "--package"
  "markdown-tui-explorer" ]` skips the workspace-sibling
  `mermaid-text` bin so the output cleanly carries
  `bin/markdown-reader`.

  The dev shell (`nix develop`) brings in `rustc`, `cargo`,
  `rustfmt`, `clippy`, `cargo-deny`, `cargo-audit` — same tools
  CI uses, so contributors don't have to set them up locally.

- **`.github/workflows/nix.yml`** — runs `nix flake check` plus
  `nix build .#markdown-reader` on `ubuntu-latest` AND
  `macos-latest` for every push/PR that touches a flake-relevant
  file (flake itself, Cargo files, source). macOS coverage matters
  because half our user base is on Darwin and Nix-on-Darwin
  surfaces different sandbox bugs than Nix-on-Linux. Cached via
  `magic-nix-cache-action` so repeat builds are fast. Smoke-tests
  the resulting binary with `--help`.

- README updated with the Nix install path next to Homebrew + AUR
  + cargo. New `docs/RELEASING-NIX.md` explains the rolling-update
  model (Nix users get whatever's on master, version-pin via their
  own `flake.lock`) so we don't have to do anything per-release.

## [1.18.1] - 2026-04-22

### Added

- **AUR (Arch Linux User Repository) packaging**. Once the `-bin`
  package is registered (one-time manual step — see
  `docs/RELEASING-AUR.md`), Arch users can install with
  `yay -S markdown-reader-bin` (or any AUR helper). Closes the
  Arch distribution gap relative to `mdt` (which ships in pacman).
  - Templates: `packaging/aur/PKGBUILD-bin.tmpl` and
    `packaging/aur/SRCINFO-bin.tmpl` — both rendered together by
    `scripts/render-aur-pkgbuild.sh`. We hand-template `.SRCINFO`
    rather than relying on `makepkg --printsrcinfo` so non-Arch
    maintainers can publish without a container or local Arch
    install.
  - New release-workflow job `publish-aur` runs on every `v*` tag.
    Same `HAS_KEY`-guarded no-op-when-missing pattern as
    `publish-homebrew`, so an unconfigured fork stays green. When
    `AUR_SSH_KEY` is set, the job clones `markdown-reader-bin.git`
    from `aur.archlinux.org`, renders both files, and pushes a
    `markdown-reader X.Y.Z` commit.
  - Architectures: `x86_64-unknown-linux-gnu` and
    `aarch64-unknown-linux-gnu` (the same release tarballs the
    Homebrew formula consumes).
  - README updated with the AUR install path next to the existing
    Homebrew + cargo paths.

### Internal

- New `docs/RELEASING-AUR.md` with the one-time AUR account / SSH
  key / first-publish setup, plus the steps for setting up the CI
  secret to enable auto-publish on every release.

## [1.18.0] - 2026-04-22

### Added

- **Stdin piping**. `cat README.md | markdown-reader` (or any pipe
  source) now opens the streamed markdown directly in the viewer.
  Closes a real workflow gap and matches `mdt`'s `cat README.md |
  mdt` ergonomics. Implementation: when stdin is detected as a pipe
  (`std::io::stdin().is_terminal() == false`), the input is drained
  to a `tempfile::NamedTempFile` with a `.md` suffix, and that path
  is used as the initial focused tab. The CLI path argument is
  ignored in this mode. The temp file is cleaned up on exit.

  On Unix, file descriptor 0 is then re-pointed at `/dev/tty` via
  `dup2(2)` so crossterm can still read keyboard input — without
  this, every key read would return EOF and the TUI would deadlock.
  Windows uses Win32 console APIs directly so no redirect is
  needed there.

### Internal

- Added `IsTerminal` import + `drain_stdin_to_temp` /
  `redirect_stdin_to_tty` helpers in `src/main.rs`.
- 1 new test (`drain_stdin_writes_md_temp_file_with_content`)
  exercises the file-creation half (mocking global stdin in a unit
  test is awkward; the FFI half is best-tested via integration
  scripts which we don't have a harness for yet).

## [1.17.3] - 2026-04-22

### Changed

- **Link picker (`f`) now sorts by TARGET heading position, not by
  where the link text was written.** The user-reported "wrong order"
  was a sort-key mismatch: the picker was strictly source-ordered,
  which meant an intro paragraph's "see also: [last section]" link
  landed at picker position [1] even though its target was at the
  END of the document. Pressing `j/k` then jumped wildly across
  sections instead of walking the doc top-to-bottom.

  After the fix, the picker reads like a navigation index — the
  order matches the order users would encounter the destinations
  if they scrolled through the document. Concrete impact on the
  user's `personal_notes.md`: the picker's first 10 entries now
  match the visible TOC structure (System overview →
  One-sentence description → Big picture diagram → ...) instead of
  starting with three intro-paragraph links pointing at
  end-of-document sections.

  Tie-breaker: when two links resolve to the same heading, source
  position breaks the tie deterministically.

### Added

- `open_link_picker_intro_links_to_end_sort_to_bottom` — direct
  regression test for the user-reported scenario.
- Updated `open_link_picker_lists_links_by_target_position` (was
  `..._in_source_order`) to assert the new target-order behaviour.

## [1.17.2] - 2026-04-22

### Fixed

- **Link picker (`f`) now lists every link in source order, including
  ones pointing at headings with inline code or special characters.**
  The user-reported "wrong order" was actually two underlying bugs in
  the heading-anchor slugifier that caused TOC links to silently drop
  out of the picker:

  1. **Inline code in headings produced empty anchors.** The
     `Event::Code(text)` handler in the markdown renderer pushed a
     styled span but didn't append `text` to `heading_text` while
     inside a heading. So `### \`kg.nodes\`` slugged to `""` instead
     of `kgnodes`, and the TOC link `[\`kg.nodes\`](#kgnodes)` failed
     `has_target`. Fixed: `Event::Code` now appends to `heading_text`
     when `in_heading` is true.

  2. **Underscores were stripped from slugs.** `char::is_alphanumeric()`
     returns false for `_`, so `### \`foo_bar\`` slugged to `foobar`
     instead of `foo_bar`. TOC links of the form
     `[\`foo_bar\`](#foo_bar)` (a common pattern) failed `has_target`.
     Fixed: `_` is now in the keep-set alongside `-` and ` `.

  3. **Consecutive hyphens were collapsed.** GitHub's slugifier
     preserves them — `# A / B` slugs to `a--b` (each space becomes
     its own hyphen, slash drops). Our slugifier collapsed them to
     `a-b`, breaking links to multi-segment headings like
     `### \`x\` / \`y\` / \`z\``. Fixed: removed the collapse loop.

  Concrete impact on the user's `personal_notes.md` (1605 lines, 70
  internal links, heavy use of `### \`kg.foo\`` headings): the picker
  was silently dropping every `kg.*` and `search.*` TOC entry.
  After the fix, all 7 inline-code anchors at TOC positions [11]-[17]
  appear in correct source order between "Table shapes" and "Who
  writes."

### Added

- 5 new tests for the slugifier:
  `heading_with_inline_code_produces_correct_anchor`,
  `heading_mixing_text_and_inline_code_includes_both_in_anchor`,
  `heading_with_underscores_preserves_underscores_in_anchor`,
  `heading_with_multi_code_and_slash_produces_correct_anchor`,
  `anchor_consecutive_spaces_preserve_hyphens` (replaces the old
  collapse test).

### Internal

- Defensive sort + dedup-after-target-check from 1.17.1 still in
  place — they cover unrelated potential failure modes.

## [1.17.1] - 2026-04-22

### Fixed

- **Oversized text-mode mermaid diagrams no longer render as
  word-wrapped garbage in place.** When the diagram's natural width
  exceeds the viewer rect, `Paragraph` was wrapping each long line
  onto multiple terminal rows, fragmenting box-drawing chars
  (`┌──┐│└─┘`) into a 2D scatter of pieces. Now the in-place
  renderer detects overflow (max line width > rect inner width) and
  substitutes a clean placeholder that reports the natural vs
  available widths and points the user at `Enter` for the
  full-screen modal:

  ```
  ┌──────────────────────────────────────────────────────┐
  │                                                      │
  │     Mermaid diagram too wide to display in place    │
  │                                                      │
  │  Natural width: 142 cells, available: 78            │
  │                                                      │
  │            Press Enter to open in fullscreen        │
  │                                                      │
  └──────────────────────────────────────────────────────┘
  ```

  The full-screen modal continues to handle the same diagram fine
  via h_scroll/v_scroll. Only the in-place display changed —
  diagrams that fit are unaffected.

- **Link picker (`f`) is more defensive about source order.** Two
  small changes guarantee top-to-bottom ordering even if a future
  refactor breaks the underlying invariant:
  1. Sort the link list by `(line, col_start)` before iteration —
     a no-op when the input is already in source order, a guard
     otherwise.
  2. Move the `has_target` (anchor exists) check **before** the
     dedup check. Previously a missing-target link could claim its
     anchor in the dedup set and silently shadow a later
     same-anchor link that DID have a target.

### Added

- 5 new tests:
  - `open_link_picker_lists_links_in_source_order`
  - `open_link_picker_handles_lists_and_mixed_structures`
  - `open_link_picker_dedup_after_target_check`
  - `max_line_display_width_handles_empty_and_unicode`
  - `max_line_display_width_counts_unicode_box_drawing_correctly`

## [1.17.0] - 2026-04-22

### Added

- **Full-screen Mermaid modal** — press `Enter` on a mermaid block to
  open it in a 90% × 90% overlay with full-screen real estate. Solves
  the "diagram too big to read" problem that plagued large flowcharts,
  state machines, and dependency graphs.
  - **Image mode**: ratatui-image's `Resize::Fit(None)` now has the
    full terminal to work with (vs. the in-document slot's
    `max_height` cap of ~30 cells). Most diagrams jump from "blob you
    can't read" to "actually legible" without any new code path.
  - **Text mode**: same `h_scroll` / `v_scroll` viewport pattern as
    the existing table modal, so wide ASCII diagrams pan instead of
    getting clipped to the right edge.
  - **Source / Failed / Pending fallbacks**: each renders into the
    same modal frame with mode-appropriate footer text (e.g. "render
    failed: {msg}"), so the user sees something meaningful regardless
    of cache state.
  - **Live cache reads**: the renderer never caches the entry into
    `MermaidModalState` — a background image render that finishes
    while the modal is open lights up on the next frame.

  Keybindings mirror the table modal exactly so muscle memory carries
  over: `j/k/h/l` (1 step), `d/u`/`PageUp`/`PageDown` (½-page),
  `g+g` (top), `G` (bottom), `0/$` (h-pan to edges), `H/L` (½-width
  h-step), `q/Esc/Enter` to close. Mouse: scroll wheel pans, click
  outside closes.

  Block resolution mirrors the table modal: prefer the mermaid block
  the cursor is inside; otherwise fall back to the first one
  intersecting the viewport. The `Enter` viewer handler tries table
  first then mermaid (mutually exclusive — only one modal opens).

### Internal

- New `Focus::MermaidModal` variant + `MermaidModalState` (5-field
  struct: `tab_id`, `block_id`, `source`, `h_scroll`, `v_scroll`).
- New `src/app/mermaid_modal.rs` (open + key + mouse handlers
  mirroring `table_modal.rs`).
- New `src/ui/mermaid_modal.rs` (renderer with image/text/source/
  pending dispatch + `slice_str_at` helper for grapheme-aware
  horizontal slicing).
- Tab switches close the mermaid modal (consistent with the table
  modal's tab-switch close behaviour).
- File reload closes the mermaid modal when the reloaded tab is the
  one the modal was opened on (stale `block_id`).
- 9 new tests cover open-under-cursor, fall-back-to-viewport,
  no-block no-op, close-on-q/Esc/Enter, scroll arithmetic with
  saturation, and `gg` / `0` resets. Plus 3 unit tests for the
  unicode-aware `slice_str_at` helper.

## [1.16.5] - 2026-04-22

### Internal

- **CI green again on stable 1.95.** Three classes of breakage,
  all build-tooling rather than user-visible:
  - **Clippy** (6 errors): `collapsible_match` × 4 in
    `key_handlers.rs` and `renderer.rs` (lifted nested `if`s into
    match guards), `explicit_counter_loop` × 2 (`(N..).zip(iter)`
    pattern), `manual_checked_division` × 1 in `table_render.rs`
    (`checked_div` instead of guarded division).
  - **Rustfmt**: drift from incremental edits picked up by the new
    `cargo fmt --all -- --check` gate. Re-formatted, no semantic
    changes.
  - **cargo-deny**: two transitive `unmaintained` advisories from
    `syntect`'s deps (`bincode 1.3.3` /
    [`RUSTSEC-2025-0141`](https://rustsec.org/advisories/RUSTSEC-2025-0141)
    and `yaml-rust 0.4.5` /
    [`RUSTSEC-2024-0320`](https://rustsec.org/advisories/RUSTSEC-2024-0320))
    started failing the build. Both lack a safe upgrade
    upstream — added narrow ignores in `deny.toml` with reason
    comments + a quarterly re-audit reminder. The advisories
    surface in `cargo audit` regardless; that job is
    `continue-on-error: true`.

## [1.16.4] - 2026-04-22

### Fixed

- **Nested-list rendering: each child bullet now gets its own line.**
  Previously, the FIRST nested item under each parent was concatenated
  to the parent's line (e.g. `• System overview ◦ One-sentence
  description` on one line, with subsequent siblings indented
  correctly on their own lines). Visible on any markdown TOC with
  nested bullets — including the user-reported `personal_notes.md`
  case. The bug was in `Tag::Item`: it didn't flush the parent's
  still-open content line before pushing the nested bullet.
  Subsequent nested items rendered correctly because the prior
  sibling's `TagEnd::Item` flushed for them. New regression test
  asserts each of 7 items in a 2-level nested list lands on its
  own line and contains no other items' text.

## [1.16.3] - 2026-04-22

### Fixed

- **Edge labels for parallel and multi-outgoing edges stack
  cleanly off the arrow row** (via `mermaid-text` 0.14.2).
  Visible on the README CI/CD pipeline (`pass` above the arrow,
  `skip` below) and the canonical TD state machine
  (`done`/`error` share a single row instead of stacking).
  Free upgrade.

## [1.16.2] - 2026-04-22

### Fixed

- **`mermaid-text` README's "Demo" Input/Output section no longer
  double-renders the same diagram.** 1.16.0's auto-detect was
  catching the Input block (`graph LR; A → B → C → D`) and
  rendering it as Mermaid even though it was meant to display the
  literal source. Tagged the Input as ` ```text ` so it stays raw,
  paired with the existing Output block that shows the rendered
  result.

## [1.16.1] - 2026-04-22

### Removed

- **Dropped the per-block "Rendered output" dogfood code blocks
  from `crates/mermaid-text/README.md`.** They were added in 1.16.0
  to make the README readable in viewers without Mermaid support,
  but in viewers that do render Mermaid (the TUI's auto-detect, our
  own image pipeline, GitHub web) every diagram appeared twice —
  once rendered, once as text below. The dogfood goal is better
  served by the existing CLI quickstart (`mermaid-text < diagram.mmd`)
  and the architecture-diagram comparison block (which stays — it
  showcases the sugiyama backend's alternative output, not a
  duplicate of the mermaid source).

## [1.16.0] - 2026-04-22

### Added

- **Untagged ` ``` ` fences whose first line declares a Mermaid
  diagram now auto-render as Mermaid blocks** (instead of falling
  through to plain code-block display). The detection is tight to
  avoid false positives:
  - `graph` / `flowchart` must be followed by an explicit direction
    token (`TD`, `TB`, `BT`, `LR`, `RL`).
  - Other declarations (`sequenceDiagram`, `stateDiagram-v2`,
    `erDiagram`, `pie`, `gantt`, `journey`, `mindmap`, `timeline`,
    `quadrantChart`, `classDiagram`, `gitGraph`, `requirement`,
    `C4*`) must be the entire first line, with documented
    exceptions for `pie title`, `pie showData`, `gantt dateFormat`.
  - Plain code with a leading `graph = {}` or natural prose like
    `"sequenceDiagram is great"` stays a code block.

  Catches the common authoring mistake of writing ` ``` ` instead
  of ` ```mermaid `, which silently broke rendering of two diagrams
  in `mermaid-text`'s own README until 1.16.0. Both readme blocks
  now also have explicit `mermaid` tags as belt-and-suspenders.

### Changed

- **`mermaid-text` README ships with rendered text-output blocks
  below every Mermaid example.** The README now eats its own dog
  food — every diagram source is followed by the text-mode output
  `mermaid-text` produces, so the README reads correctly in any
  viewer (GitHub, terminal, plain-text grep) regardless of whether
  the viewer renders Mermaid.

## [1.15.1] - 2026-04-22

### Fixed

- **Sugiyama-backend chrome glitches reduced** (via `mermaid-text`
  0.14.1). The architecture-diagram opt-in now has wider inter-
  layer gaps and cleaner junctions. Free upgrade.

## [1.15.0] - 2026-04-22

### Added

- **Sugiyama layout backend (opt-in)** for flat dependency graphs
  (via `mermaid-text` 0.14.0). The mermaid-text CLI gains a
  `--sugiyama` flag and `RenderOptions::backend` for embedded
  callers. Better crossing minimisation + Brandes-Köpf coordinate
  assignment + long-edge dummy nodes via the [`ascii-dag`] crate.
  Default behaviour unchanged — `Native` remains the default
  backend until subgraph and parallel-edge support land in the
  Sugiyama wrapper.

### Changed

- MSRV bumped to 1.92 to match `ascii-dag`'s minimum.

[`ascii-dag`]: https://crates.io/crates/ascii-dag

## [1.14.0] - 2026-04-22

### Fixed

- **Subgraph labels in mixed-direction diagrams have breathing room
  from the border** (via `mermaid-text` 0.13.0). Phase 3 of the
  parallel-edge work: `direction TB` subgraphs inside an `LR` graph
  (and vice versa) widen their bounds when they contain
  parallel-edge labels, with the layered layout pre-allocating the
  same extra space so external nodes don't collide. Visible on the
  README Supervisor (`creates`/`panics`) example. Free upgrade.

## [1.13.5] - 2026-04-22

### Fixed

- **TD/BT state diagrams with cycles render their back-edge entry
  cleanly** (via `mermaid-text` 0.12.2). The garbled `├┤` glyph
  pair at the back-edge source is now a proper L-corner (`├┘`
  for TD, `├┐` for BT). Visible on the canonical README state
  machine. Free upgrade.

## [1.13.4] - 2026-04-22

### Fixed

- **erDiagram relationships now visually connect their entity boxes**
  (via `mermaid-text` 0.12.1). The cardinality glyphs and label
  used to float in a detached row below both boxes — readers had
  to mentally connect them to the entities above. Now the line
  sits at the entity-name row of both boxes, merging into the
  side borders via `┤` and `├` tee glyphs. The README CUSTOMER↔ORDER
  example reads as a single diagram instead of two stacked artefacts.
  Free upgrade.

## [1.13.3] - 2026-04-22

### Fixed

- **Cramped parallel-edge labels in flowcharts and state diagrams
  finally have breathing room** (via `mermaid-text` 0.12.0). When
  two or more labelled edges connect the same node pair (CI/CD's
  `pass`/`skip`, Supervisor's `creates`/`panics`, state diagrams
  with `done`/`task` bidirectional pairs), the inter-layer gap
  now widens to give each label its own row (LR/RL) or column
  (TD/BT). Closes ROADMAP items #2 + #4. Free upgrade.

## [1.13.2] - 2026-04-22

### Fixed

- **State diagrams with back-edges read much more clearly** (via
  `mermaid-text` 0.11.2). The back-edge in cyclic diagrams (most
  TD state machines) now routes around the perimeter instead of
  threading through the diagram body — forward edges and their
  labels stay in clean channels. Free upgrade.

## [1.13.1] - 2026-04-22

### Changed

- **`erDiagram` visual polish** (via `mermaid-text` 0.11.1). Phase
  2 of the erDiagram series: entity boxes now render with attribute
  tables inside (type / name / keys columns), and relationship
  arrows carry single-character cardinality glyphs at each endpoint
  (`1`, `?`, `+`, `*`). Free upgrade.

## [1.13.0] - 2026-04-22

### Added

- **`erDiagram` support** in markdown mermaid blocks (via
  `mermaid-text` 0.11.0). The most-requested missing diagram
  type per ROADMAP now renders natively. Phase 1 — entity-name
  boxes in source-order row, relationships drawn as labelled
  arrows with `1:N` style cardinality summaries, dashed lines for
  non-identifying (`..`) relationships.
- Phase 2 (attribute tables + crow's-foot cardinality glyphs)
  and Phase 3 (grid layout) ship in subsequent `mermaid-text`
  0.11.x releases. Free upgrade — no markdown-reader code
  changes.

## [1.12.1] - 2026-04-22

### Changed

- **Crossing-minimisation hardening** in flowchart and state
  diagrams (via `mermaid-text` 0.10.1). Adds median + transpose
  passes alongside the existing barycenter sweep — no visible
  change on the current gallery (barycenter alone was already
  optimal on these diagrams) but produces tighter layouts on
  pathologically dense graphs that older code would settle into
  sub-optimal local minima. Free upgrade.

## [1.12.0] - 2026-04-22

### Changed

- **Long-edge routing in flowchart and state diagrams** (via
  `mermaid-text` 0.10.0). Edges spanning more than one layer now
  get per-intermediate-layer waypoints, giving them a near-
  straight channel through the layout instead of detouring
  around intervening nodes. Phase A.1 of the layered-layout
  improvements series; Phases A.2 (Brandes-Köpf compaction) and
  A.3 (median + transpose crossing min) ship in subsequent
  `mermaid-text` 0.10.x releases.
- **Source-breaking for external consumers of `mermaid-text`**:
  `layered::layout` now returns `LayoutResult` instead of a
  position `HashMap`; `render::render` gains a fourth parameter
  for waypoints. No surface-level changes in markdown-reader
  itself — bumped to 1.12.0 to reflect the dep's minor bump.

## [1.11.7] - 2026-04-22

### Changed

- **Sequence-diagram polish** in markdown mermaid blocks (via
  `mermaid-text` 0.9.7): bottom participant boxes mirror the top
  (matches Mermaid's bracketed-lifeline convention), and block
  tags split into two `[…]` brackets (`╔═[alt]══[cache hit]═══╗`
  instead of `╔═[alt: cache hit]═══╗`) to match Mermaid's
  badge-plus-condition style. Free upgrade — no markdown-reader
  code changes.

## [1.11.6] - 2026-04-22

### Changed

- **Mermaid TD/BT diagrams: arrow tips merge into destination box
  borders** (via `mermaid-text` 0.9.6). Previously `▾` sat one row
  above each `┌────┐` top border, creating a visible gap in TUI
  display. Now renders as `┌─▾─┐` — the arrow visually connects
  to the box. LR/RL flows already had no gap (cell adjacency).
  Free upgrade — no markdown-reader code changes.

## [1.11.5] - 2026-04-22

### Fixed

- **Edge labels no longer puncture node or subgraph borders** in
  flowchart and state diagrams (via `mermaid-text` 0.9.5). The
  Supervisor pattern's `panics` label inside Factory's bottom
  border, the keyboard-lock state diagram's `EvNumLockPressed`
  overwriting node corners, and similar issues across five state-
  diagram snapshots are all fixed. Free upgrade — no
  markdown-reader code changes.

## [1.11.4] - 2026-04-21

### Added

- **`pie` chart support** in markdown mermaid blocks (via
  `mermaid-text` 0.9.4). First new diagram type since
  `sequenceDiagram`. Renders as a horizontal bar chart with
  optional title and optional `showData` value column. Free
  upgrade — no markdown-reader code changes.

## [1.11.3] - 2026-04-21

### Added

- **Sequence-diagram block statements** in markdown mermaid blocks
  (via `mermaid-text` 0.9.3). `loop`/`alt`/`opt`/`par`/`critical`/
  `break` and their continuation keywords (`else`/`and`/`option`)
  render as labelled rectangles spanning the columns of inner
  messages, with proper nesting and inset for nested blocks.
  Completes the four-part sequence-diagram polish series. Free
  upgrade — no markdown-reader code changes.

## [1.11.2] - 2026-04-21

### Added

- **Sequence-diagram activation bars** in markdown mermaid blocks
  (via `mermaid-text` 0.9.2). Both `activate X` / `deactivate X`
  directives and the inline `A->>+B` / `B-->>-A` shorthand render
  as heavy `┃` overlays on participant lifelines. Free upgrade —
  no markdown-reader code changes.

## [1.11.1] - 2026-04-19

### Added

- **Sequence-diagram notes** in markdown mermaid blocks (via
  `mermaid-text` 0.9.1). `note left of X : text`,
  `note right of X : text`, `note over X : text`, and the
  multi-anchor `note over X,Y : text` form all render now —
  rounded boxes anchored to participant columns. `<br>` /
  `<br/>` in note text becomes a real line break. Free upgrade —
  no markdown-reader code changes.

## [1.11.0] - 2026-04-20

### Added

- **`autonumber` directive in mermaid sequence diagrams** (via
  `mermaid-text` 0.9.0). API call sequences in markdown files now
  show `[1]`, `[2]`, `[3]` … prefixes when the source has
  `autonumber`. Mid-diagram re-base (`autonumber 100`) and pause
  (`autonumber off`) both honoured. Free upgrade — no
  markdown-reader code changes.
- Foundation data model for the rest of sequence-diagram polish
  (notes, activation bars, block brackets); those features land
  in subsequent `mermaid-text` 0.9.x releases.

## [1.10.1] - 2026-04-20

### Added

- **Notes anchored to states** in mermaid state diagrams (via
  `mermaid-text` 0.8.1). `note left of X : text`,
  `note right of X : text`, `note over X : text`, plus the
  multi-line `note left of X / … / end note` form. Each note
  renders as a small rounded box connected to its anchor by a
  dotted, no-arrow line. Free upgrade — no markdown-reader code
  changes.

## [1.10.0] - 2026-04-20

### Added

- **`classDef`, `class`, and `:::className` shorthand** for both
  mermaid flowcharts and state diagrams (via `mermaid-text` 0.8.0).
  Define a colour palette once with `classDef cache fill:#234,…`
  then apply it across many states with `class A,B,C cache` or
  inline (`A:::cache --> B:::warn`). Subgraphs / composite states
  coloured via `class CompositeId styleName` get a coloured
  border. Free upgrade — no markdown-reader code changes; the
  call into `mermaid_text::render_with_width` already passes
  `--color` through.
- **`style` and `linkStyle` now apply to state diagrams** (they
  worked for flowcharts since 0.4.0; were silently skipped for
  state diagrams until now).

## [1.9.2] - 2026-04-20

### Added

- **State diagrams now render `<<choice>>`, `<<fork>>`, and
  `<<join>>` shape modifiers** (via `mermaid-text` 0.7.2). Choice
  points show as decision diamonds; fork / join synchronisation
  bars render as thick lines perpendicular to the flow direction
  (vertical `┃` in LR layouts, horizontal `━━━` in TB). State
  diagrams with branch points (auth flows, Sagas,
  retry-with-conditional) and parallel-flow synchronisation (CI
  orchestration, distributed fan-out / fan-in) now read correctly
  instead of as a chain of identical rounded boxes.

## [1.9.1] - 2026-04-20

### Fixed

- **Edge labels in mermaid diagrams no longer overwrite node interior
  text.** Picks up `mermaid-text` 0.7.1 which expanded the label
  placement candidate set and added a node-interior collision check.
  The user's circuit-breaker FSM rendering used to show a stray `5`
  inside the OPEN state (from the edge label `5 consecutive failures`
  spilling onto the box content); now the label lands on a clean row
  below the segment and OPEN's content is intact.

## [1.9.0] - 2026-04-20

### Changed

- **Mermaid state diagrams now default to `LR` direction.** In a text
  canvas, TB (Mermaid's spec default) inserts `layer_gap` blank rows
  between each row of nodes, so a typical 4-state chain balloons into
  40+ rows — most of it empty. LR keeps the chain horizontal. The
  user's circuit-breaker FSM drops from ~52 rows to ~11 rows. Users
  who want the old layout can add `direction TB` to the diagram
  source. Bumps `mermaid-text` to 0.7.0.

## [1.8.2] - 2026-04-20

### Fixed

- **Scrolling inside a tall mermaid diagram now works.** v1.8.1 stopped
  the layout from clamping the reserved height, but the text-mode
  renderer (`AsciiDiagram`, `SourceOnly`, `Failed`) still always drew
  the diagram from line 0 of the text — `Paragraph::new(text)` ignores
  scroll offsets — so the user saw the top of the diagram pinned in
  place no matter how far they scrolled into it. Now the renderer
  slices the diagram lines by the scroll offset before passing them to
  `Paragraph`, mirroring the `DocBlock::Text` path. Tall composite
  state diagrams scroll smoothly through their full height.

## [1.8.1] - 2026-04-20

### Fixed

- **Tall mermaid diagrams are no longer cut off.** Text-mode diagrams
  (the `AsciiDiagram` cache variant — anything rendered through
  figurehead / `mermaid-text`) used to be clamped to
  `mermaid_max_height` (default 30 lines) when sizing their layout slot.
  A composite-state diagram or any flowchart taller than 30 lines had
  its bottom rows silently unreachable: scrolling moved past the
  reserved region into the next document block instead of revealing
  more of the diagram. Layout now reserves the diagram's actual line
  count, with a 1000-line defensive safety cap. `mermaid_max_height`
  still applies to image renders and source-text fallbacks where the
  bound is meaningful.

## [1.8.0] - 2026-04-20

### Added

- **Mermaid state diagrams now render inline.** `stateDiagram` and
  `stateDiagram-v2` blocks in markdown files are rendered as Unicode
  box-drawing art (previously fell back to showing the raw source).
  Includes `[*]` start/end markers, transitions with labels,
  `STATE : description` accumulation, `state "Display" as Id`, and
  per-diagram direction overrides.
- **Composite states `state X { … }`** with recursive nesting and
  per-composite `[*]` scope render as nested rounded rectangles.
  External edges to / from composite IDs are automatically rewritten
  to land on the composite's inner start / end marker so the arrow
  connects visibly to the composite border region.
- Bumped `mermaid-text` dependency to **0.6.0**.

### Fixed

- **Back-edge perimeter paths now visibly connect to their boxes.**
  Any flowchart (or state diagram) with a back-edge (`C --> A` when
  `A` is upstream of `C`) previously rendered the perimeter line and
  arrow tip with a 1-cell gap to each node's border. `mermaid-text`
  0.6.0 stamps `┬`/`┴` (or `├`/`┤` for TD/BT) junction glyphs at both
  ends so the connection reads cleanly. Surfaces constantly in retry
  loops in state diagrams.

## [1.7.1] - 2026-04-17

### Added
- **`mermaid-text` library crate** (`crates/mermaid-text/`). A standalone
  MIT Rust library that renders Mermaid flowcharts as Unicode box-drawing
  text — no browser, no image protocols, pure Rust. Supports
  `graph`/`flowchart` with LR/TD/RL/BT directions, node shapes
  (rectangle, rounded, diamond, circle), edge labels, and Sugiyama-style
  layered layout. Published as a workspace member; will be released as
  an independent crate.
- **Text-mode mermaid rendering** via `mermaid-text`. Flowcharts in
  Text mode or on non-graphics terminals render as Unicode art instead
  of raw source. Sequence/state/class diagrams still fall back to source
  (Phase 2-3 of `mermaid-text`).
- **Visible block cursor** at `(cursor_line, cursor_col)`. A single-cell
  highlight in `accent` colour shows the exact horizontal position in
  both normal and visual modes, making `h`/`l` movement and `v`
  character selection visually trackable.

### Fixed
- **Mermaid cache invalidated on resize.** Cached `AsciiDiagram` text
  is fixed-width; resizing the viewer now clears the mermaid cache so
  diagrams re-render at the new width.
- **Flowchart parser skips mermaid keywords.** `subgraph`, `direction`,
  `end`, `style`, `classDef`, `click`, `linkStyle` are no longer
  treated as node definitions. `<br/>` tags are stripped from labels.

## [1.7.0] - 2026-04-17

### Added
- **Mermaid rendering settings.** Press `c` → Mermaid section to choose
  Auto / Text / Image rendering mode. `mermaid_max_height` in
  config.toml caps diagram height (default 30 lines, was hardcoded 50).
- **`has_limited_rendering` diagrams (state diagrams) now try
  text-mode rendering** instead of falling through to raw source.
  Infrastructure for `AsciiDiagram` cache variant is in place; the
  text renderer is currently stubbed (the only candidate — figurehead
  0.4.3 — has fatal bugs for TUI use: debug prints, panics, freezes).

### Fixed
- **Link picker (`f`) now updates the cursor.** Selecting a heading
  via `f` jumped the scroll but left `cursor_line` at its old position.
  The next `j`/`k` would snap back to the pre-jump location. Now uses
  `cursor_line + scroll_to_cursor_centered` like all other jumps.
- **Stale mermaid image results no longer overwrite text-mode entries.**
  After switching rendering mode, in-flight image tasks that complete
  are discarded if the cache entry is no longer `Pending`.

## [1.6.4] - 2026-04-17

### Fixed
- **Mermaid renders no longer peg the CPU.** Added a 30-second timeout
  per render and a cap of 2 concurrent render tasks.
  `mermaid-rs-renderer` is pre-1.0 and can hang on certain diagram
  types; previously a hung render would run forever at 100% CPU.  With
  multiple diagrams queued (e.g. after a theme change clears the
  cache), every core could be saturated.  Now: hung renders time out
  cleanly (the diagram shows an error footer), and at most 2 render
  threads run simultaneously.

### Changed
- **Compact tree indentation.** Reduced per-level indent from 2 spaces
  to 1 space and switched expand/collapse markers from `▼`/`▶` to
  the narrower `▾`/`▸`.  At depth 5, filenames now start 5 characters
  earlier — enough to show the full name on most terminals instead of
  truncating.

## [1.6.2] - 2026-04-17

### Fixed
- **Duplicate key events on Windows.** crossterm emits both
  `KeyEventKind::Press` and `KeyEventKind::Release` for every keystroke
  on Windows; the event loop was forwarding both, causing every action
  to fire twice. Now only `Press` events are forwarded. No effect on
  macOS/Linux (they only emit `Press`). Fixes #1.

## [1.6.1] - 2026-04-17

### Changed
- **Code quality: zero clippy pedantic warnings.** Eliminated all 181
  pedantic lint warnings across the codebase: 62 integer-cast warnings
  resolved via new saturating-cast helpers in `src/cast.rs`
  (`u32_sat`, `u16_sat`, `u16_from_u32`); 19 infallible casts replaced
  with `From` trait calls; remaining 100 warnings fixed mechanically
  (redundant closures, `let...else`, inlined format vars, merged match
  arms, items-before-statements, etc.).
- **Module split: `app.rs` (4093 lines) → `src/app/` (7 files,
  largest 1009 lines).** Key handlers, search, file operations, yank,
  table-modal logic, and tests each live in focused submodules.
  `App` struct and top-level dispatch stay in `mod.rs`.
- **Module split: `markdown_view.rs` (2000 lines) → `src/ui/markdown_view/`
  (8 files, largest 528 lines).** Draw, state, highlight, mermaid draw,
  gutter, visual-row math, and tests each in their own file.
- **All production `unwrap()` calls replaced** with `let Some(...) else { return }` guards.

## [1.6.0] - 2026-04-17

### Added
- **Character-wise visual mode (`v`).** Press `v` in the viewer to
  start a character-level selection. `h`/`l`/`Left`/`Right` move the
  cursor horizontally within the line; `j`/`k`/`d`/`u`/`gg`/`G` move
  vertically and clamp the column to the new line's width. `y` yanks
  the exact character range to the clipboard; `Esc`/`v` cancels.
  First/last lines of the selection are partially highlighted; middle
  lines are fully highlighted. Spans are split at column boundaries
  preserving per-span styles.
- **Horizontal cursor (`cursor_col`).** The viewer now tracks a
  column position within the current logical line. `h`/`l` move it
  left/right. The status bar shows `col N` so the position is always
  visible.
- **Line-wise visual mode is now `V`** (uppercase, was also `V`
  before) and shows `VISUAL LINE` in the status bar. `v` (lowercase)
  is character-wise and shows `VISUAL`. Matches vim convention.

### Changed
- `VisualRange` now carries `mode` (`Char`/`Line`), `anchor_col`,
  and `cursor_col` fields alongside the existing line fields.
  `char_range_on_line` is the single method callers use to determine
  highlighting — no mode-branching in the rendering pipeline.

## [1.5.3] - 2026-04-17

### Fixed
- **Search-jump now lands on the correct line.** `logical_line_at_source`
  was returning the *last* logical line whose source number matched the
  target, but the same source line can appear at multiple rendered
  positions (heading + trailing blank, list End-event dip back to the
  list's start line). The last occurrence is a rendering artifact; the
  first is the actual content. Now exact matches return the first
  occurrence immediately. Approximate matches (target inside a joined
  paragraph) still scan the full vector for the closest preceding line.

## [1.5.2] - 2026-04-17

### Fixed
- **Cursor no longer jumps back to line 1 on Linux.** On Linux,
  `inotify` fires `IN_ACCESS` events when a file is read (not just
  modified). Our 500ms-debounced file watcher treated those as changes,
  triggering a reload that reset the cursor and scroll to 0. Now
  `reload_changed_tabs` compares the new content against the existing
  `tab.view.content` and skips the reload when nothing actually changed.
  Genuine reloads also preserve the cursor position (clamped to the new
  document length) instead of always resetting to line 1.
- **`markdown-reader path/file.md` now opens the file immediately.**
  Previously, passing a file path (instead of a directory) produced an
  empty tree because the app used the file itself as the tree root.
  Now the root is set to the file's parent directory, the tree is
  populated normally, and the file is opened in a tab on startup.
- **Borderless viewer when the file tree is hidden.** Pressing
  `Shift+H` to hide the tree now also removes the viewer's outer
  border, giving the markdown content full terminal width and height.
  `[` and `]` (tree width adjustment) are no-ops while the tree is
  hidden. Pressing `Shift+H` again restores both the tree and the
  border.

### Changed
- `App::new` now takes an optional `initial_file: Option<PathBuf>`
  parameter for the file-path-as-argument feature.

## [1.5.1] - 2026-04-17

### Fixed
- **File-tree discovery is dramatically faster on large repos.** The
  recursive per-directory walker (`max_depth(1)` + re-recurse) was
  re-reading and re-compiling `.gitignore` matchers at every directory
  level, which scaled pathologically on monorepos with deep trees.
  Replaced with a single `ignore::WalkBuilder::build_parallel()` pass
  that amortises the ignore-matcher cost across worker threads, then
  folds the flat path list into a sorted `FileEntry` tree.

## [1.5.0] - 2026-04-17

### Added
- **LaTeX math rendering.** Inline math (`$...$`) and display math
  (`$$...$$`) are now parsed via pulldown-cmark's `ENABLE_MATH` option
  and rendered as Unicode-approximated text. Greek letters (`α`, `β`,
  `π`, …), operators (`∑`, `∫`, `∇`, `∞`, …), fractions (`a/b`),
  square roots (`√(x)`), and super/subscripts (`x²`, `xᵢ`) display
  as readable Unicode. Display math renders in a bordered block
  labelled `math`, mirroring the code-block style. Zero new
  dependencies — pure Rust string conversion in `src/markdown/math.rs`.

## [1.4.3] - 2026-04-16

### Fixed
- **Table modal preserved only the first span's colour when slicing for
  horizontal scroll.** The first span on every row is the left border
  `│` styled with `table_border`, so the whole row — including cell
  text and header text — inherited the border's muted colour, making
  the modal unreadable on every theme. `slice_line_at` now walks the
  line span-by-span, keeping each span's original style, and only
  replaces a span's content with the correct display-width slice.
  Double-width characters straddling the left edge are still
  replaced with a single space so column alignment stays consistent.

## [1.4.2] - 2026-04-16

### Changed
- **Trimmed transitive dependencies.** Dropped `image-defaults` from
  `ratatui-image` and `default-features` from `image` — we only use the
  `RgbaImage`/`DynamicImage` types to shuttle pixels from `tiny_skia`
  (mermaid rasterization) to `ratatui-image`, never to decode image
  files. Removing the format decoders also removes the
  `ravif → rav1e → bitstream-io → core2` chain that was triggering a
  "yanked dependency" warning on every build. Significantly smaller
  compile time and binary. No functional change.

## [1.4.1] - 2026-04-16

### Fixed
- **`Enter` now expands the table under the cursor** rather than the first
  table that happens to intersect the viewport.  Falls back to the
  first-visible table when the cursor is on prose, preserving the old
  "click anywhere to expand" behaviour.
- **Table modal contrast** — the expanded-table modal's grid borders
  were rendered with a colour tuned for the main viewer background but
  drawn against the modal's tinted background, which made the grid
  barely visible on light themes (GitHub Light in particular).  The
  modal body now uses the viewer background directly; the focused-border
  colour around the outer frame still signals "this is a modal".

### Changed
- README now includes screenshots (viewer overview, global search,
  GitHub Light with settings) and lists all eight themes in the
  Features section (Solarized Light and Gruvbox Light were missing from
  the count).  The settings-modal keybinding description mentions the
  new "search preview" option.

## [1.4.0] - 2026-04-16

### Added
- **Global search modal.** Press `/` in the Tree or Viewer to open a
  full-screen search pane. Results are grouped per file with a match
  count and a preview of the first match (full-line or ~80-char
  snippet, selectable in Settings). `j`/`k`/arrows/`Ctrl+n`/`Ctrl+p`
  navigate; `Enter` opens the selected file in a new tab; `Tab`
  toggles between Files and Content modes; `Esc` dismisses. Click a
  row to open it, click outside to dismiss.
- **Smartcase search.** Lowercase query = case-insensitive match;
  any uppercase character in the query = case-sensitive. An `Aa`
  / `aA` indicator in the modal footer shows the active mode. No
  manual toggle required.
- **Jump to match line on open.** Confirming a content-search result
  opens the file and places the viewer cursor on the first-match
  source line, centred in the viewport.
- **Tree auto-expand on open.** Whenever a file is opened
  programmatically (search, link follow, session restore), the file
  tree expands any collapsed ancestor directories so the file's row
  is visible and selected.
- **Vim-style visual-line mode in the viewer.** Press `V` to start a
  line-wise selection; `j`/`k`/`d`/`u`/`gg`/`G`/`PageDown`/`PageUp`
  extend the range; `y` yanks the selection to the clipboard via
  OSC 52 and exits; `Esc` or `V` cancels. Status bar shows
  `VISUAL` while active. `yy` in normal mode copies the current
  cursor line.
- **Search preview setting.** New `Search preview` section in the
  Settings modal toggles between Full line (default) and Snippet
  (~80 chars) previews in the search modal. Persisted in
  `config.toml` as `search_preview`.
- **Cursor position in the status bar.** The status bar now shows
  `(cursor_line / total_lines, percentage)` so `d`/`u`/`gg`/`G`
  navigation is reflected immediately. (Already shipped in 1.3.0;
  this release adds the `VISUAL` label override.)

### Fixed
- **GitHub Light theme: invisible tab and status-bar labels.** The
  `accent` and `selection_fg` colors in the GitHub Light palette
  were both the same blue, so text drawn on an accent background
  (active tab name, focus indicator) rendered blue-on-blue and
  vanished. A new `Palette::on_accent_fg` field disambiguates the
  two roles; for GitHub Light it's set to white.
- **Search-jump to the right source line inside lists and
  paragraphs.** Previously the inverse source-to-logical mapping
  assumed `source_lines` was monotonically non-decreasing, but
  pulldown-cmark's End-of-list events can cause dips (e.g.
  `[..., 165, 160, 167, ...]`), leading to wrong jumps for any
  match whose target line lived after a list. The scan now walks
  the full vector and returns the last candidate whose source
  `<= target`.
- **Gutter line numbers now align with wrapped content.** The
  gutter paragraph previously rendered one number per logical
  line against a wrapping content paragraph, so the two drifted
  vertically on long lines. The gutter now emits blank
  continuation rows that match the content's wrap count, so a
  line number always sits next to its content.
- **Table header source-line tracking.** pulldown-cmark does not
  emit `Tag::TableRow` for a table's header — cells live directly
  inside `Tag::TableHead` — so the header's source line was
  recorded as 0 regardless of the table's actual position. Now
  captured from `Tag::TableHead`'s own span.
- **`pending_jump` no longer leaks on read failure.** A new
  `Action::FileLoadFailed` variant fires when the async read
  fails, clearing the pending jump so a later unrelated file
  load cannot inherit a stale target.
- **Misleading search-truncation footer.** The "N more" count was
  derived by subtracting a file cap from a match count. Replaced
  with a clear `"results capped at N files"` message.

### Changed
- **`:N` go-to-line now centres the target** to match the UX of
  search-result jumps. Both are long-distance jumps; neither
  should park the cursor at the viewport edge.
- **Content search counts all matches per file.** Previously the
  search broke after the first match in each file; the new
  modal needs the count for its per-file display.
- **`edtui` upgraded to 0.11.2** (already in 1.2.0) now with
  `default-features = false` to drop the `arboard` clipboard
  dependency we do not use. Smaller binary, headless-safe.

## [1.3.0] - 2026-04-15

### Fixed
- **Doc-search navigation now moves the viewer cursor.** `n`/`N` and the
  auto-jump to the first match were mutating `scroll_offset` directly,
  leaving `cursor_line` stranded at its old position. Press `j`/`k`
  after `n` now moves the cursor from the match row, as expected.
- **Cursor highlight no longer disappears over tables and mermaid
  blocks.** The highlight code now runs for `DocBlock::Text`,
  `DocBlock::Table`, and the source-text fallback of `DocBlock::Mermaid`
  via a shared `patch_cursor_highlight()` helper. Mermaid blocks in
  image mode render a 1-row background bar beneath the image so the
  cursor is still visible around the image padding.
- **Entering edit mode inside a table or mermaid block lands on the
  correct source line.** `source_line_at` previously returned only the
  block's opening line, so `i` from the middle of a 20-row table dropped
  you on the header. Tables now track per-row source lines via a new
  `TableBlock::row_source_lines` vector populated from
  pulldown-cmark's `OffsetIter`. Mermaid blocks interpolate as
  `fence + 1 + K`, clamped to the content length — same pattern code
  blocks already use for their content rows.

### Added
- **Cursor position in the viewer status bar.** The status bar now
  shows `(cursor_line / total_lines, percentage)` instead of the old
  scroll-based percentage, so `d`/`u`/`gg`/`G`/`PageDown`/`PageUp`
  navigation is reflected immediately even when the cursor stays
  on-screen.

## [1.2.0] - 2026-04-15

### Added
- **Visible viewer cursor.** The viewer now shows a highlighted cursor row
  (background from `palette.selection_bg`, carries through line wrapping)
  that moves with `j`/`k`/`d`/`u`/`PageDown`/`PageUp`/`gg`/`G`. Scroll
  follows the cursor when it would leave the viewport, so the observable
  behaviour of "press `j` to scroll down" is preserved while unlocking a
  proper notion of "where I am" for future features.
- **Vim-style edit mode** via
  [edtui](https://crates.io/crates/edtui) 0.11.2. Press `i` in the viewer
  to drop into a modal editor at the exact source line of the viewer
  cursor. Normal/Insert/Visual modes with vim motions (`w`, `b`, `e`,
  `gg`, `G`, `0`, `$`, `dd`, `yy`, `p`, etc.). `:w` saves atomically
  (tempfile + rename), `:q` returns to the rendered view, `:wq` does
  both, `:q!` force-discards unsaved changes. Undo/redo via `u` /
  `Ctrl+r`. The editor theme tracks the active UI palette.
- **Source-line plumbing through the renderer.** pulldown-cmark byte
  offsets are now threaded through `MdRenderer` so every rendered logical
  line reports its originating source line. `DocBlock::Text` carries a
  parallel `source_lines: Vec<u32>`; `DocBlock::Mermaid` and `TableBlock`
  carry `source_line: u32`. This is what powers exact cursor-to-editor
  positioning and unlocks future line-aware features.
- **Git status refresh on save.** Editing a file and pressing `:w` now
  recolors its entry in the file tree immediately — new files turn
  yellow (modified) as soon as the write lands, no git poll wait.

### Changed
- `j`/`k`/`d`/`u`/`PageDown`/`PageUp`/`gg`/`G` in the viewer now move a
  cursor rather than the scroll offset directly. Scroll follows cursor,
  so the visible effect is the same — but the cursor is the new primary
  concept for "where I am".
- `edtui` is pulled in with `default-features = false` to avoid the
  `arboard` clipboard dependency. Our app handles mouse and clipboard
  separately, and this keeps the binary smaller and headless-safe.

### Fixed
- Mouse events are now ignored while `Focus::Editor` is active, so clicks
  in the tree panel during editing no longer select and open files.

## [1.1.0] - 2026-04-14

### Added
- **Syntax highlighting for fenced code blocks.** Fenced blocks with a
  language tag (`rust`, `python`, `javascript`, `go`, `json`, `bash`, and
  many more) are now tokenised and colored inline. Implemented via
  [syntect](https://crates.io/crates/syntect) with the pure-Rust
  `default-fancy` feature — no C dependencies, no onig. Each UI theme
  maps to a bundled syntect theme so colors track the active palette.
- **Table modal mouse support.** The full-screen table viewer (`Enter`
  on a table) now responds to the mouse wheel: plain scroll pans rows,
  `Shift`+scroll pans columns, and clicking outside the modal closes it.
- **Column-boundary horizontal panning in the table modal.** `h` and `l`
  now snap to the previous/next column boundary rather than moving a
  single cell at a time. `H` and `L` pan half a page instead of a fixed
  ten cells, making wide tables dramatically faster to navigate.
- **`scroll_left` / `scroll_right` (`MouseEventKind::ScrollLeft` /
  `ScrollRight`)** are handled where terminals emit them, mapping to
  one-column-boundary pans.

### Fixed
- **Code block right-border alignment.** Lines containing multi-byte
  characters (em dashes, CJK, emoji) no longer push the box frame out of
  alignment. Width measurement now uses `unicode-width` display cells
  throughout instead of byte length.

### Changed
- `render_markdown` and `MarkdownViewState::load` now take the active
  `Theme` so fenced code blocks can be highlighted with a matching
  syntect theme. Callers inside the crate are updated accordingly.

[1.1.0]: https://github.com/leboiko/markdown-reader/releases/tag/v1.1.0
