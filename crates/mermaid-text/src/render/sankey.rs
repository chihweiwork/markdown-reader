//! Renderer for [`Sankey`] diagrams.
//!
//! ## Layout
//!
//! Phase 1 uses a grouped-arrow list layout. Source nodes are printed as
//! header lines; each outgoing arc is indented below with an arrow glyph
//! and the flow value:
//!
//! ```text
//! Sankey: Energy Flow
//!
//! Agricultural waste
//!   ──[124.7]──► Bio-conversion
//!
//! Bio-conversion
//!   ──[0.6]────► Liquid
//!   ──[280.3]──► Solid
//!
//! Coal imports
//!   ──[11.6]───► Coal
//!
//! Coal
//!   ──[75.6]───► Solid
//! ```
//!
//! Source nodes are listed in first-seen order from the flow list. Nodes that
//! appear only as targets (leaf nodes) are not printed as headers — they have
//! no outgoing flows to list.
//!
//! ## Phase 1 limitations
//!
//! True proportional sankey rendering — node heights scaled to total flow
//! volume and curvilinear bands between them — requires Sugiyama layout with
//! sankey-specific band routing. This is planned for a future phase.
//! The current renderer deliberately stays readable and correct rather than
//! proportional.
//!
//! ## max_width
//!
//! When `max_width` is `Some(n)`, node header lines and arrow label text
//! are truncated to fit within the budget. The minimum guaranteed width is
//! 20 columns; narrower budgets are silently clamped up.

use unicode_width::UnicodeWidthStr;

use crate::sankey::Sankey;

/// Default column width when no budget is specified.
const DEFAULT_WIDTH: usize = 80;

/// Minimum column budget (clamp floor for very narrow terminals).
const MIN_WIDTH: usize = 20;

/// Glyph used as the arrow shaft filler.
const SHAFT: &str = "\u{2500}"; // ─
/// Glyph used as the arrowhead pointing right.
const ARROW_HEAD: &str = "\u{25BA}"; // ►

/// Render a [`Sankey`] to a Unicode string.
///
/// # Arguments
///
/// * `diag`      — the parsed diagram
/// * `max_width` — optional column budget; lines are truncated to this many
///   columns (minimum [`MIN_WIDTH`])
///
/// # Returns
///
/// A multi-line string ready for printing. Trailing newlines are stripped.
pub fn render(diag: &Sankey, max_width: Option<usize>) -> String {
    let width = max_width
        .map(|w| w.max(MIN_WIDTH))
        .unwrap_or(DEFAULT_WIDTH);

    let mut out = String::new();

    if diag.flows.is_empty() {
        out.push_str("(empty sankey diagram)");
        return out;
    }

    // Collect outgoing flows per source, preserving first-seen source order.
    // We use a Vec of (source, Vec<(target, value)>) to keep insertion order.
    let mut sources: Vec<String> = Vec::new();
    let mut outgoing: std::collections::HashMap<String, Vec<(String, f64)>> =
        std::collections::HashMap::new();

    for flow in &diag.flows {
        if !sources.contains(&flow.source) {
            sources.push(flow.source.clone());
        }
        outgoing
            .entry(flow.source.clone())
            .or_default()
            .push((flow.target.clone(), flow.value));
    }

    // Determine the maximum value width (for bracket alignment).
    // We format values as one decimal place, so find the longest formatted
    // value to decide how wide the `[value]` column needs to be.
    let max_val_len = diag
        .flows
        .iter()
        .map(|f| format!("{:.1}", f.value).len())
        .max()
        .unwrap_or(1);

    let first = true;
    let mut first_source = first;
    for source in &sources {
        // Blank line between source groups (but not before the very first).
        if !first_source {
            out.push('\n');
        }
        first_source = false;

        // Source header line — truncated to max_width.
        let header = truncate_to_width(source, width);
        out.push_str(&header);
        out.push('\n');

        let arcs = outgoing.get(source).map(Vec::as_slice).unwrap_or(&[]);
        for (target, value) in arcs {
            let arc_line = format_arc(target, *value, max_val_len, width);
            out.push_str(&arc_line);
            out.push('\n');
        }
    }

    // Strip trailing newline.
    while out.ends_with('\n') {
        out.pop();
    }

    out
}

/// Format a single arc line.
///
/// Shape: `  ──[<value>]──► <target>`
///
/// The `[value]` slot is right-padded with `─` shaft characters so that
/// all arrowheads in the same source group align at the same column. The
/// total line length is clamped to `max_width`.
///
/// `max_val_len` is the number of digits needed by the longest value in the
/// entire diagram (used to align arrowheads across all source groups).
fn format_arc(target: &str, value: f64, max_val_len: usize, max_width: usize) -> String {
    // `  ──[` prefix is 5 chars wide.
    const INDENT: &str = "  ";
    const OPEN_SHAFT: &str = "\u{2500}\u{2500}["; // ──[

    let value_str = format!("{value:.1}");

    // Right-pad the value with spaces to `max_val_len` so values align.
    // Then close the bracket: `]──► `.
    let pad = max_val_len.saturating_sub(value_str.len());
    let bracket_content = format!("{value_str}{}", " ".repeat(pad));

    // Shaft after bracket: `──►` (3 unicode chars = 3 cols).
    // We use a short fixed suffix so the arrow stands out.
    let suffix = format!("{SHAFT}{SHAFT}{ARROW_HEAD} ");

    let prefix = format!("{INDENT}{OPEN_SHAFT}{bracket_content}]{suffix}");
    let prefix_w = UnicodeWidthStr::width(prefix.as_str());

    // Remaining budget for the target label.
    let remaining = max_width.saturating_sub(prefix_w);
    let target_truncated = truncate_to_width(target, remaining);

    format!("{prefix}{target_truncated}")
}

/// Truncate `s` so its display width does not exceed `max_cols`.
///
/// Uses `unicode-width` for accurate terminal column counting.
/// If truncation is needed, the last visible character is replaced with `…`.
fn truncate_to_width(s: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let total = UnicodeWidthStr::width(s);
    if total <= max_cols {
        return s.to_string();
    }
    // Need to truncate. Reserve one column for `…`.
    let budget = max_cols.saturating_sub(1);
    let mut result = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > budget {
            break;
        }
        result.push(ch);
        used += cw;
    }
    result.push('\u{2026}'); // …
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::sankey::parse;

    fn canonical_src() -> &'static str {
        "sankey-beta

%% source,target,value
Agricultural 'waste',Bio-conversion,124.729
Bio-conversion,Liquid,0.597
Bio-conversion,Solid,280.322
Coal imports,Coal,11.606
Coal,Solid,75.571"
    }

    #[test]
    fn source_nodes_appear_as_headers() {
        let diag = parse(canonical_src()).unwrap();
        let out = render(&diag, None);

        // Source nodes that have outgoing flows must appear as header lines.
        assert!(
            out.contains("Bio-conversion"),
            "Bio-conversion header missing:\n{out}"
        );
        assert!(
            out.contains("Coal imports"),
            "Coal imports header missing:\n{out}"
        );
        assert!(out.contains("Coal\n"), "Coal header missing:\n{out}");
    }

    #[test]
    fn arrow_glyphs_present() {
        let diag = parse(canonical_src()).unwrap();
        let out = render(&diag, None);

        // Arrow head glyph must appear on arc lines.
        assert!(
            out.contains(ARROW_HEAD),
            "arrowhead glyph missing:\n{out}"
        );
        // Shaft glyph must be present.
        assert!(out.contains(SHAFT), "shaft glyph missing:\n{out}");
    }

    #[test]
    fn all_target_names_appear_in_output() {
        let diag = parse(canonical_src()).unwrap();
        let out = render(&diag, None);

        for name in &["Liquid", "Solid", "Coal"] {
            assert!(
                out.contains(name),
                "target {name:?} missing from output:\n{out}"
            );
        }
    }

    #[test]
    fn values_appear_in_output() {
        let diag = parse(canonical_src()).unwrap();
        let out = render(&diag, None);

        // Spot-check formatted values.
        assert!(
            out.contains("124.7"),
            "124.7 value missing from output:\n{out}"
        );
        assert!(
            out.contains("0.6"),
            "0.6 value missing from output:\n{out}"
        );
        assert!(
            out.contains("280.3"),
            "280.3 value missing from output:\n{out}"
        );
    }

    #[test]
    fn empty_sankey_renders_placeholder() {
        let diag = Sankey::default();
        let out = render(&diag, None);
        assert!(
            out.contains("empty"),
            "empty placeholder missing:\n{out}"
        );
    }

    #[test]
    fn max_width_truncates_long_names() {
        let src = "sankey-beta\nA Very Long Source Node Name That Exceeds Eighty Columns,B,10.0";
        let diag = parse(src).unwrap();
        let out = render(&diag, Some(40));

        for line in out.lines() {
            let w = UnicodeWidthStr::width(line);
            assert!(
                w <= 40,
                "line exceeds max_width=40 (w={w}): {line:?}"
            );
        }
    }

    #[test]
    fn single_flow_round_trip() {
        let src = "sankey-beta\nSource,Target,42.5";
        let diag = parse(src).unwrap();
        let out = render(&diag, None);

        assert!(out.contains("Source"), "source missing");
        assert!(out.contains("Target"), "target missing");
        assert!(out.contains("42.5"), "value missing");
    }
}
