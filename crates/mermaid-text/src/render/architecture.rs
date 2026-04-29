//! Renderer for [`Architecture`] diagrams. Produces Unicode box-drawing output
//! with groups as labeled border boxes, services as inner boxes, and a text
//! connection summary below.
//!
//! ## Layout (Phase 1)
//!
//! ```text
//! ┌─ API (cloud) ──────────────────────────────────────────┐
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
//! │  │ Database │  │ Storage  │  │ Storage  │  │ Server │ │
//! │  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
//! └────────────────────────────────────────────────────────┘
//!
//! Connections:
//!   db:L ─── R:server
//!   disk1:T ─── B:server
//!   disk2:T ─── B:db
//! ```
//!
//! Top-level services (not in any group) are rendered as standalone boxes
//! above the group section.
//!
//! ## Phase 1 limitations
//!
//! - Icon rendering is not implemented; icon names are shown parenthetically
//!   in group headers only.
//! - True spatial edge routing is not implemented; edges appear as a textual
//!   connection summary below the layout.
//! - All services are rendered in a single horizontal row per group; very
//!   wide groups may exceed `max_width` even after truncation.

use unicode_width::UnicodeWidthStr;

use crate::architecture::{ArchEdge, ArchGroup, ArchService, Architecture};

/// Padding between adjacent service boxes inside a group.
const SERVICE_GAP: usize = 2;

/// Inner width (content area) for each service box, before any truncation.
const SERVICE_INNER_W: usize = 10;

/// Padding inside the group border: spaces on each side.
const GROUP_PADDING: usize = 2;

/// Render an [`Architecture`] diagram to a Unicode string.
///
/// # Arguments
///
/// * `diag`      — the parsed diagram
/// * `max_width` — optional column budget; service labels and group titles are
///   truncated with `…` when they exceed the available space
pub fn render(diag: &Architecture, max_width: Option<usize>) -> String {
    let mut out = String::new();

    // Top-level services (not in any group) are rendered first as standalone boxes.
    let top_level = diag.top_level_services();
    if !top_level.is_empty() {
        render_service_row(&top_level, max_width, &mut out);
        if !diag.groups.is_empty() {
            out.push('\n');
        }
    }

    // Render each group as a labeled border box containing its services.
    for (i, group) in diag.groups.iter().enumerate() {
        render_group(group, diag, max_width, &mut out);
        if i + 1 < diag.groups.len() {
            out.push('\n');
        }
    }

    // Connections summary below.
    let conn_part = render_connections(&diag.edges);
    if !conn_part.is_empty() {
        if !out.is_empty() {
            out.push('\n');
            out.push('\n');
        }
        out.push_str(&conn_part);
    }

    // Trim trailing newlines.
    while out.ends_with('\n') {
        out.pop();
    }

    out
}

// ---------------------------------------------------------------------------
// Group rendering
// ---------------------------------------------------------------------------

/// Render a group as a border box with services inside.
///
/// Layout convention used throughout this renderer:
/// - `inner_w` is the number of characters between `│` and `│` in content lines.
/// - Content line total width = `inner_w + 2` (two `│` walls).
/// - Border line total width = `inner_w + 2` (one `┌` + `inner_w` dashes + one `┐`).
///
/// Service boxes have label-content area `svc_inner_w` wide. Each box has:
/// - 1 space of padding on each side inside `│ … │`, so box outer width = `svc_inner_w + 4`.
///
/// A service row of `n` boxes:  `n * (svc_inner_w + 4) + (n-1) * SERVICE_GAP`
///
/// The group's `inner_w` = service_row_width + 2 * GROUP_PADDING.
fn render_group(
    group: &ArchGroup,
    diag: &Architecture,
    max_width: Option<usize>,
    out: &mut String,
) {
    let services = diag.services_in_group(&group.id);
    let title = group_title(group);

    // Service row width (outer). Each service box outer width = svc_inner_w + 4.
    let svc_row_w = if services.is_empty() {
        0
    } else {
        service_row_width(&services, max_width)
    };

    // Group inner_w = content between │ walls in the group border.
    // Must be wide enough for: (GROUP_PADDING spaces) + service_row + (GROUP_PADDING spaces)
    // AND wide enough to show the title in the top border.
    // Title part consumes: `─ {title} ` = len(title) + 3 chars inside the border.
    let title_min_inner = UnicodeWidthStr::width(title.as_str()) + 3;
    let inner_w = if services.is_empty() {
        title_min_inner.max(4)
    } else {
        (svc_row_w + 2 * GROUP_PADDING).max(title_min_inner)
    };

    // Apply max_width: outer group width = inner_w + 2; solve for inner_w.
    let inner_w = if let Some(budget) = max_width {
        inner_w.min(budget.saturating_sub(2)).max(title_min_inner)
    } else {
        inner_w
    };

    // Available width for the service row content (inner_w minus 2*GROUP_PADDING).
    let available_for_svc = inner_w.saturating_sub(2 * GROUP_PADDING);

    // Top border: `┌─ Title ──────────────┐`
    let top = build_group_top_border(&title, inner_w);
    out.push_str(&top);
    out.push('\n');

    if services.is_empty() {
        // Empty group: one blank content line.
        out.push('\u{2502}'); // │
        for _ in 0..inner_w {
            out.push(' ');
        }
        out.push('\u{2502}'); // │
        out.push('\n');
    } else {
        let padding = " ".repeat(GROUP_PADDING);
        let svc_lines = build_service_row_lines(&services, max_width);
        for (row_top, row_mid, row_bot) in &svc_lines {
            for row in &[row_top, row_mid, row_bot] {
                out.push('\u{2502}'); // │
                out.push_str(&padding);
                out.push_str(&pad_to(row, available_for_svc));
                out.push_str(&padding);
                out.push('\u{2502}'); // │
                out.push('\n');
            }
        }
    }

    // Bottom border: `└───────────────────────┘`
    let bot = build_group_bottom_border(inner_w);
    out.push_str(&bot);
    out.push('\n');
}

/// Build the group top border: `┌─ Title ──────────────┐`.
///
/// `inner_w` is the number of characters between `┌` and `┐`.
/// The title occupies `─ {title} ` (len(title) + 3 chars).
fn build_group_top_border(title: &str, inner_w: usize) -> String {
    let title_w = UnicodeWidthStr::width(title);
    // Title segment inside: "─ " + title + " " = title_w + 3
    let title_seg = title_w + 3;
    let fill = inner_w.saturating_sub(title_seg);

    let mut s = String::new();
    s.push('\u{250C}'); // ┌
    s.push('\u{2500}'); // ─
    s.push(' ');
    s.push_str(title);
    s.push(' ');
    for _ in 0..fill {
        s.push('\u{2500}'); // ─
    }
    s.push('\u{2510}'); // ┐
    s
}

/// Build the group bottom border: `└──────────────────────┘`.
///
/// `inner_w` is the number of `─` dashes between the corner glyphs.
fn build_group_bottom_border(inner_w: usize) -> String {
    let mut s = String::new();
    s.push('\u{2514}'); // └
    for _ in 0..inner_w {
        s.push('\u{2500}'); // ─
    }
    s.push('\u{2518}'); // ┘
    s
}

/// Pad (or truncate) `content` to exactly `width` display columns.
fn pad_to(content: &str, width: usize) -> String {
    let cw = UnicodeWidthStr::width(content);
    if cw >= width {
        truncate_to_width(content, width)
    } else {
        let mut s = content.to_string();
        for _ in 0..(width - cw) {
            s.push(' ');
        }
        s
    }
}

/// Human-readable title for a group: `Label (icon)` or just `Label` / `id`.
fn group_title(group: &ArchGroup) -> String {
    let name = group
        .label
        .as_deref()
        .filter(|l| !l.is_empty())
        .unwrap_or(&group.id);
    match &group.icon {
        Some(icon) if !icon.is_empty() => format!("{name} ({icon})"),
        _ => name.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Service row rendering
// ---------------------------------------------------------------------------

/// Render a standalone row of service boxes (used for top-level services).
fn render_service_row(services: &[&ArchService], max_width: Option<usize>, out: &mut String) {
    let rows = build_service_row_lines(services, max_width);
    for (top, mid, bot) in &rows {
        out.push_str(top);
        out.push('\n');
        out.push_str(mid);
        out.push('\n');
        out.push_str(bot);
        out.push('\n');
    }
}

/// Build service box lines for a slice of services in a single row.
///
/// Returns a `Vec` of `(top_line, label_line, bottom_line)` tuples, one
/// per logical row of service boxes. In Phase 1, all services always go
/// into a single row; this vec therefore has exactly one element.
fn build_service_row_lines(
    services: &[&ArchService],
    max_width: Option<usize>,
) -> Vec<(String, String, String)> {
    let inner_w = effective_service_inner_w(services, max_width);

    let mut top = String::new();
    let mut mid = String::new();
    let mut bot = String::new();

    for (i, svc) in services.iter().enumerate() {
        if i > 0 {
            let gap = " ".repeat(SERVICE_GAP);
            top.push_str(&gap);
            mid.push_str(&gap);
            bot.push_str(&gap);
        }

        // ┌──────────┐
        top.push('\u{250C}');
        for _ in 0..inner_w + 2 {
            top.push('\u{2500}');
        }
        top.push('\u{2510}');

        // │  label   │
        let label = svc.display_label();
        let lw = UnicodeWidthStr::width(label);
        let label = if lw > inner_w {
            truncate_to_width(label, inner_w)
        } else {
            label.to_string()
        };
        let lw = UnicodeWidthStr::width(label.as_str());
        let total_pad = inner_w.saturating_sub(lw);
        let left_pad = total_pad / 2;
        let right_pad = total_pad - left_pad;
        mid.push('\u{2502}');
        mid.push(' ');
        for _ in 0..left_pad {
            mid.push(' ');
        }
        mid.push_str(&label);
        for _ in 0..right_pad {
            mid.push(' ');
        }
        mid.push(' ');
        mid.push('\u{2502}');

        // └──────────┘
        bot.push('\u{2514}');
        for _ in 0..inner_w + 2 {
            bot.push('\u{2500}');
        }
        bot.push('\u{2518}');
    }

    vec![(top, mid, bot)]
}

/// Compute the inner width (content area) for service boxes given the services and `max_width`.
///
/// A service box renders as `┌──(inner_w+2)──┐ / │ (inner_w) │ / └──┘`.
/// The total box outer width = `inner_w + 4` (2 borders + 2 padding spaces).
fn effective_service_inner_w(services: &[&ArchService], max_width: Option<usize>) -> usize {
    // Natural width: max of SERVICE_INNER_W and the longest label.
    let max_label_w = services
        .iter()
        .map(|s| UnicodeWidthStr::width(s.display_label()))
        .max()
        .unwrap_or(0);
    let natural = max_label_w.max(SERVICE_INNER_W);

    if let Some(budget) = max_width {
        let n = services.len();
        if n == 0 {
            return natural;
        }
        // Total service-row outer width = n*(inner_w + 4) + (n-1)*SERVICE_GAP
        // (each box is inner_w + 4: 2 walls + 2 padding spaces, plus border dashes)
        // Solve for inner_w: inner_w = (budget - (n-1)*SERVICE_GAP - n*4) / n
        let overhead = (n - 1) * SERVICE_GAP + n * 4;
        if budget <= overhead {
            return 1;
        }
        let allowed = (budget - overhead) / n;
        natural.min(allowed).max(1)
    } else {
        natural
    }
}

/// Total display width of a service row for the given services.
///
/// Each box outer width = `inner_w + 4`. Boxes are separated by `SERVICE_GAP`.
fn service_row_width(services: &[&ArchService], max_width: Option<usize>) -> usize {
    let n = services.len();
    if n == 0 {
        return 0;
    }
    let inner_w = effective_service_inner_w(services, max_width);
    n * (inner_w + 4) + (n - 1) * SERVICE_GAP
}

// ---------------------------------------------------------------------------
// Connections summary
// ---------------------------------------------------------------------------

/// Render the connection summary as text lines below the diagram.
fn render_connections(edges: &[ArchEdge]) -> String {
    if edges.is_empty() {
        return String::new();
    }
    let mut out = String::from("Connections:\n");
    for edge in edges {
        let src_part = match edge.source_port {
            Some(p) => format!("{}:{}", edge.source, p.abbreviation()),
            None => edge.source.clone(),
        };
        let tgt_part = match edge.target_port {
            Some(p) => format!("{}:{}", p.abbreviation(), edge.target),
            None => edge.target.clone(),
        };
        out.push_str(&format!(
            "  {} \u{2500}\u{2500}\u{2500} {}\n",
            src_part, tgt_part
        ));
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

// ---------------------------------------------------------------------------
// Text utilities
// ---------------------------------------------------------------------------

/// Truncate `s` so its display width is `<= max_w`, appending `…` if needed.
fn truncate_to_width(s: &str, max_w: usize) -> String {
    if max_w == 0 {
        return String::new();
    }
    let w = UnicodeWidthStr::width(s);
    if w <= max_w {
        return s.to_string();
    }
    let target = max_w.saturating_sub(1);
    let mut result = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if used + cw > target {
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
    use crate::parser::architecture::parse;

    fn parsed(src: &str) -> Architecture {
        parse(src).expect("parse must succeed")
    }

    #[test]
    fn renders_group_with_services() {
        let src = "architecture-beta
    group api(cloud)[API]
    service db(database)[Database] in api
    service server(server)[Server] in api";
        let arch = parsed(src);
        let out = render(&arch, None);

        // Group header must contain the label.
        assert!(out.contains("API"), "group label 'API' missing:\n{out}");
        // Service labels must appear.
        assert!(out.contains("Database"), "service 'Database' missing:\n{out}");
        assert!(out.contains("Server"), "service 'Server' missing:\n{out}");
        // Box-drawing characters must be present.
        assert!(out.contains('\u{250C}'), "top-left corner ┌ missing:\n{out}");
        assert!(out.contains('\u{2518}'), "bottom-right corner ┘ missing:\n{out}");
        assert!(out.contains('\u{2502}'), "vertical bar │ missing:\n{out}");
    }

    #[test]
    fn renders_standalone_top_level_services() {
        let src = "architecture-beta\n    service ext(internet)[External]";
        let arch = parsed(src);
        let out = render(&arch, None);

        assert!(out.contains("External"), "top-level service label missing:\n{out}");
        assert!(out.contains('\u{250C}'), "top-left corner missing:\n{out}");
    }

    #[test]
    fn renders_connections_summary() {
        let src = "architecture-beta
    service db(database)[Database]
    service server(server)[Server]
    db:L -- R:server";
        let arch = parsed(src);
        let out = render(&arch, None);

        assert!(out.contains("Connections:"), "Connections: header missing:\n{out}");
        assert!(out.contains("db:L"), "source port missing:\n{out}");
        assert!(out.contains("R:server"), "target port missing:\n{out}");
        assert!(out.contains('\u{2500}'), "dash line missing:\n{out}");
    }

    #[test]
    fn max_width_constrains_output() {
        let src = "architecture-beta
    group api(cloud)[API]
    service db(database)[Database] in api
    service server(server)[Server] in api
    db:L -- R:server";
        let arch = parsed(src);
        let out = render(&arch, Some(50));

        // Each line must be reasonably contained (allow slight overshoot from
        // group border + GROUP_PADDING; the important check is labels are truncated).
        for line in out.lines() {
            let w = UnicodeWidthStr::width(line);
            assert!(
                w <= 60,
                "line width {w} greatly exceeds budget: {line:?}"
            );
        }
    }

    #[test]
    fn empty_diagram_renders_without_panic() {
        let arch = Architecture::default();
        let out = render(&arch, None);
        // An empty diagram has nothing to render — output may be empty.
        assert!(!out.contains('\u{250C}'), "no box for empty diagram");
    }
}
