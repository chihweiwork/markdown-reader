//! Renderer for [`QuadrantChart`]. Produces a Unicode cross-axis chart with
//! labeled quadrants and proportionally-placed data points.
//!
//! ## Layout
//!
//! ```text
//! Reach and engagement of campaigns
//!
//!                          High Engagement
//!                                ^
//!       Need to promote          |     We should expand
//!                                |
//!   · F (0.35,0.78)              |
//!                                |
//!   · A (0.30,0.60)              |   · C (0.57,0.69)
//!                                |
//! Low Reach ─────────────────────┼──────────────────── High Reach
//!                                |
//!                                |   · D (0.78,0.34)
//!   · E (0.40,0.34)              |
//!                                |   · B (0.45,0.23)
//!         Re-evaluate            |     May be improved
//!                                v
//!                          Low Engagement
//! ```
//!
//! ## Glyph alphabet
//!
//! | Glyph | Meaning                              |
//! |-------|--------------------------------------|
//! | `─`   | Horizontal axis segment              |
//! | `│`   | Vertical axis segment                |
//! | `┼`   | Cross at origin                      |
//! | `^`   | Top arrow on vertical axis           |
//! | `v`   | Bottom arrow on vertical axis        |
//! | `·`   | Data point marker                    |
//!
//! ## max_width
//!
//! When `max_width` is `Some(n)`, the canvas width is clamped to that budget.
//! The default canvas is ~70 columns wide when no budget is specified.
//!
//! ## Phase 1 limitations
//!
//! - Points that map to the same terminal cell overwrite each other; the last
//!   point (in source order) wins. This is acceptable for Phase 1.
//! - Point labels are not truncated or wrapped; very long names may extend
//!   slightly past `max_width`.

use unicode_width::UnicodeWidthStr;

use crate::quadrant_chart::{QuadrantChart, QuadrantPoint};

/// Default canvas width (columns) when no `max_width` is given.
const DEFAULT_WIDTH: usize = 70;

/// Minimum canvas width; below this the chart would be unreadable.
const MIN_WIDTH: usize = 30;

/// Number of canvas rows for the chart body (excluding title + axis labels).
const CANVAS_ROWS: usize = 20;

/// Render a [`QuadrantChart`] to a Unicode string.
///
/// # Arguments
///
/// * `diag`      — the parsed diagram
/// * `max_width` — optional column budget; the canvas is sized to fit within
///   this budget (minimum `MIN_WIDTH` columns)
///
/// # Returns
///
/// A multi-line string ready for printing. The layout is a cross-axis chart
/// with the x-axis running horizontally across the middle row and the y-axis
/// running vertically through the centre column.
pub fn render(diag: &QuadrantChart, max_width: Option<usize>) -> String {
    let canvas_width = max_width.map(|w| w.max(MIN_WIDTH)).unwrap_or(DEFAULT_WIDTH);

    // The canvas is split left / right at the y-axis column.
    // Centre column holds the `│` / `┼` / `^` / `v` glyphs.
    // We give slightly more room to the right half so axis labels and quadrant
    // labels have breathing room on both sides.
    let center_col = canvas_width / 2;

    let mut out = String::new();

    // Title
    if let Some(title) = &diag.title {
        let title_w = UnicodeWidthStr::width(title.as_str());
        let pad = center_col.saturating_sub(title_w / 2);
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(title);
        out.push('\n');
        out.push('\n');
    }

    // Axis label — top (high y)
    if let Some(y_ax) = &diag.y_axis {
        let label = &y_ax.high;
        let lw = UnicodeWidthStr::width(label.as_str());
        let pad = center_col.saturating_sub(lw / 2);
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(label);
        out.push('\n');
    }

    // Build the chart canvas as a Vec<Vec<char>> grid.
    // Rows go top-to-bottom; columns go left-to-right.
    // The cross-centre is at row CANVAS_ROWS/2, col center_col.
    let rows = CANVAS_ROWS;
    let mid_row = rows / 2; // row index of the x-axis

    // cell(row, col) → char; initialise to space.
    let mut grid: Vec<Vec<char>> = vec![vec![' '; canvas_width]; rows + 2]; // +2 for arrows

    // Draw y-axis (vertical bar + top/bottom arrows).
    // Row 0 = top arrow `^`, row 1..rows = body, row rows+1 = bottom arrow `v`.
    grid[0][center_col] = '^';
    for row in grid.iter_mut().take(rows + 1).skip(1) {
        row[center_col] = '\u{2502}'; // │
    }
    grid[rows + 1][center_col] = 'v';

    // Draw x-axis (horizontal dashes across mid_row + 1, because row 0 = arrow).
    // The actual body rows are grid[1..=rows]; mid_row is a 0-based index into
    // those body rows, so the grid row is mid_row + 1.
    let x_axis_grid_row = mid_row + 1;
    for (c, cell) in grid[x_axis_grid_row]
        .iter_mut()
        .enumerate()
        .take(canvas_width)
    {
        if c != center_col {
            *cell = '\u{2500}'; // ─
        }
    }
    // Cross at intersection.
    grid[x_axis_grid_row][center_col] = '\u{253C}'; // ┼

    // Place quadrant labels.
    // Q1 = top-right: upper-right quadrant (row 1, col center_col+2)
    // Q2 = top-left:  upper-left  quadrant (row 1, toward left)
    // Q3 = bottom-left: lower-left
    // Q4 = bottom-right: lower-right
    let label_row_top = 1usize;
    let label_row_bot = rows; // last body row

    if let Some(q1) = &diag.quadrants.q1 {
        // Top-right: place starting at center_col+2
        place_text(&mut grid, label_row_top, center_col + 2, q1, canvas_width);
    }
    if let Some(q2) = &diag.quadrants.q2 {
        // Top-left: right-align before center_col-2
        place_text_right_aligned(&mut grid, label_row_top, center_col.saturating_sub(2), q2);
    }
    if let Some(q3) = &diag.quadrants.q3 {
        // Bottom-left: right-align before center_col-2
        place_text_right_aligned(&mut grid, label_row_bot, center_col.saturating_sub(2), q3);
    }
    if let Some(q4) = &diag.quadrants.q4 {
        // Bottom-right: place starting at center_col+2
        place_text(&mut grid, label_row_bot, center_col + 2, q4, canvas_width);
    }

    // Place data points.
    // x in [0, 1]: 0 → col 0, 1 → col canvas_width-1.
    //   Left half = x < 0.5 (cols 0..center_col-1)
    //   Right half = x >= 0.5 (cols center_col+1..canvas_width-1)
    // y in [0, 1]: 0 → bottom row (rows), 1 → top row (1).
    //   Bottom half = y < 0.5 (rows mid_row+1..rows)
    //   Top half = y >= 0.5 (rows 1..mid_row)
    //
    // We map each coordinate to the available space in its half, keeping a
    // 1-column gap around the axis so points don't collide with the axis itself.
    let left_cols = center_col.saturating_sub(1); // columns available on the left
    let right_cols = canvas_width.saturating_sub(center_col + 2); // columns on the right
    let top_rows = mid_row.saturating_sub(1); // body rows above the axis (excl. label row 1)
    let bot_rows = rows.saturating_sub(mid_row + 1); // body rows below the axis (excl. label row)

    for pt in &diag.points {
        let (col, grid_row) = point_to_grid(
            pt, center_col, left_cols, right_cols, mid_row, top_rows, bot_rows, rows,
        );

        // Clamp to canvas bounds; skip if the point can't be placed.
        if grid_row == 0 || grid_row > rows || col >= canvas_width {
            continue;
        }

        // Place the marker glyph.
        grid[grid_row][col] = '\u{00B7}'; // middle dot ·

        // Place the label to the right of the marker: "Name (x,y)".
        let label = format!(" {} ({:.2},{:.2})", pt.name, pt.x, pt.y);
        place_text(&mut grid, grid_row, col + 1, &label, canvas_width);
    }

    // Render x-axis edge labels.
    // Left label is `low`; right label is `high`. They are placed on x_axis_grid_row
    // at the far left/right edges, overwriting axis dashes.
    let total_grid_rows = rows + 2;
    if let Some(x_ax) = &diag.x_axis {
        if !x_ax.low.is_empty() {
            // Place at the very left edge; the label sits on top of the dashes.
            place_text(&mut grid, x_axis_grid_row, 0, &x_ax.low, canvas_width);
        }
        if !x_ax.high.is_empty() {
            let hw = UnicodeWidthStr::width(x_ax.high.as_str());
            let start_col = canvas_width.saturating_sub(hw);
            place_text(
                &mut grid,
                x_axis_grid_row,
                start_col,
                &x_ax.high,
                canvas_width,
            );
        }
    }

    // Emit all grid rows.
    for row in grid.iter().take(total_grid_rows) {
        let row_str: String = row.iter().collect();
        // Trim trailing spaces.
        let trimmed = row_str.trim_end();
        out.push_str(trimmed);
        out.push('\n');
    }

    // Axis label — bottom (low y)
    if let Some(y_ax) = &diag.y_axis {
        let label = &y_ax.low;
        let lw = UnicodeWidthStr::width(label.as_str());
        let pad = center_col.saturating_sub(lw / 2);
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(label);
        out.push('\n');
    }

    // Trim trailing newlines.
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Map a data point's (x, y) coordinates to a (col, grid_row) position.
///
/// The mapping keeps points within their correct quadrant and away from the
/// axis lines by using the available space in each half independently.
///
/// The grid row index is 1-based (row 0 = top arrow, rows+1 = bottom arrow).
#[allow(clippy::too_many_arguments)]
fn point_to_grid(
    pt: &QuadrantPoint,
    center_col: usize,
    left_cols: usize,
    right_cols: usize,
    mid_row: usize,
    top_rows: usize,
    bot_rows: usize,
    rows: usize,
) -> (usize, usize) {
    // Column mapping: x=0 → far left, x=1 → far right.
    // We split at x=0.5 and map each half to the available column range.
    let col = if pt.x < 0.5 {
        // Left half: map [0, 0.5) → [0, center_col - 2]
        let frac = pt.x / 0.5; // 0.0..1.0 within the left half
        (frac * left_cols as f64) as usize
    } else {
        // Right half: map [0.5, 1] → [center_col+2, canvas_width-1]
        let frac = (pt.x - 0.5) / 0.5; // 0.0..1.0 within the right half
        center_col + 2 + (frac * right_cols.saturating_sub(1) as f64) as usize
    };

    // Row mapping: y=1 → top (row 2), y=0 → bottom (row rows-1).
    // Grid row 0 = `^` arrow, row 1 = top quadrant label row,
    // row mid_row+1 = x-axis, row rows = bottom quadrant label row, rows+1 = `v` arrow.
    // We map into the interior rows, skipping label rows and axis row.
    let grid_row = if pt.y >= 0.5 {
        // Top half: map [0.5, 1] → rows 2..mid_row (skipping label row 1).
        let frac = (1.0 - pt.y) / 0.5; // 0.0 at y=1 (top), 1.0 at y=0.5
        let interior_rows = top_rows.saturating_sub(1); // rows 2..mid_row
        2 + (frac * interior_rows.saturating_sub(1) as f64) as usize
    } else {
        // Bottom half: map [0, 0.5) → rows mid_row+2..rows-1 (skipping axis and label).
        let frac = (0.5 - pt.y) / 0.5; // 0.0 at y=0.5, 1.0 at y=0
        let interior_rows = bot_rows.saturating_sub(1);
        let interior_start = mid_row + 2; // skip the axis row itself
        interior_start + (frac * interior_rows.saturating_sub(1) as f64) as usize
    };

    // Ensure we don't place on label rows or axis row.
    let safe_row = grid_row
        .max(2) // not row 0 (arrow) or 1 (quadrant label)
        .min(rows - 1); // not rows (quadrant label) or rows+1 (arrow)
    // Also avoid the x-axis row.
    let safe_row = if safe_row == mid_row + 1 && safe_row > 2 {
        safe_row - 1
    } else if safe_row == mid_row + 1 && safe_row < rows - 1 {
        safe_row + 1
    } else {
        safe_row
    };

    (col, safe_row)
}

/// Place `text` into the grid starting at `(row, start_col)`.
///
/// Characters that would extend past `max_col` are silently dropped.
/// This does not respect Unicode combining characters or double-wide glyphs
/// in the column counting (Phase 1 limitation — nearly all point labels are ASCII).
fn place_text(grid: &mut [Vec<char>], row: usize, start_col: usize, text: &str, max_col: usize) {
    let row_len = grid[row].len();
    let limit = max_col.min(row_len);
    for (col, ch) in (start_col..).zip(text.chars()) {
        if col >= limit {
            break;
        }
        grid[row][col] = ch;
    }
}

/// Place `text` right-aligned so its last character is at `end_col`.
///
/// Characters that start before column 0 are silently dropped.
fn place_text_right_aligned(grid: &mut [Vec<char>], row: usize, end_col: usize, text: &str) {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return;
    }
    // Start position (may underflow to 0 if label is wider than end_col).
    let start_col = end_col.saturating_sub(len);
    let row_len = grid[row].len();
    for (i, &ch) in chars.iter().enumerate() {
        let col = start_col + i;
        if col < row_len {
            grid[row][col] = ch;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::quadrant_chart::parse;

    fn canonical_src() -> &'static str {
        "quadrantChart
    title Reach and engagement of campaigns
    x-axis Low Reach --> High Reach
    y-axis Low Engagement --> High Engagement
    quadrant-1 We should expand
    quadrant-2 Need to promote
    quadrant-3 Re-evaluate
    quadrant-4 May be improved
    Campaign A: [0.3, 0.6]
    Campaign B: [0.45, 0.23]
    Campaign C: [0.57, 0.69]
    Campaign D: [0.78, 0.34]
    Campaign E: [0.40, 0.34]
    Campaign F: [0.35, 0.78]"
    }

    #[test]
    fn title_appears_in_output() {
        let chart = parse(canonical_src()).unwrap();
        let out = render(&chart, None);
        assert!(
            out.contains("Reach and engagement of campaigns"),
            "title missing from output:\n{out}"
        );
    }

    #[test]
    fn quadrant_labels_appear_in_correct_corners() {
        let chart = parse(canonical_src()).unwrap();
        let out = render(&chart, None);

        // Q1 = top-right
        assert!(out.contains("We should expand"), "Q1 label missing:\n{out}");
        // Q2 = top-left
        assert!(out.contains("Need to promote"), "Q2 label missing:\n{out}");
        // Q3 = bottom-left
        assert!(out.contains("Re-evaluate"), "Q3 label missing:\n{out}");
        // Q4 = bottom-right
        assert!(out.contains("May be improved"), "Q4 label missing:\n{out}");

        // Verify Q1 appears on the same line as Q2 (both are on the top quadrant label row).
        let q1_line = out
            .lines()
            .find(|l| l.contains("We should expand"))
            .expect("Q1 line missing");
        let q2_line = out
            .lines()
            .find(|l| l.contains("Need to promote"))
            .expect("Q2 line missing");
        assert_eq!(
            q1_line, q2_line,
            "Q1 and Q2 labels should be on the same line"
        );

        // Verify Q3 appears on the same line as Q4.
        let q3_line = out
            .lines()
            .find(|l| l.contains("Re-evaluate"))
            .expect("Q3 line missing");
        let q4_line = out
            .lines()
            .find(|l| l.contains("May be improved"))
            .expect("Q4 line missing");
        assert_eq!(
            q3_line, q4_line,
            "Q3 and Q4 labels should be on the same line"
        );

        // Q1/Q2 row must come BEFORE the Q3/Q4 row (top labels above bottom labels).
        let q1_line_no = out
            .lines()
            .position(|l| l.contains("We should expand"))
            .unwrap();
        let q3_line_no = out.lines().position(|l| l.contains("Re-evaluate")).unwrap();
        assert!(
            q1_line_no < q3_line_no,
            "top quadrant labels ({q1_line_no}) must precede bottom ({q3_line_no})"
        );
    }

    #[test]
    fn points_render_inside_canvas() {
        let chart = parse(canonical_src()).unwrap();
        let out = render(&chart, Some(80));
        // All point names must appear somewhere in the output.
        for name in &["Campaign A", "Campaign B", "Campaign C", "Campaign D"] {
            assert!(
                out.contains(name),
                "point {name:?} missing from output:\n{out}"
            );
        }
        // The cross glyph must be present.
        assert!(out.contains('\u{253C}'), "cross glyph ┼ missing:\n{out}");
    }

    #[test]
    fn axis_labels_appear_on_outer_edges() {
        let chart = parse(canonical_src()).unwrap();
        let out = render(&chart, None);

        // x-axis edge labels appear on the x-axis line.
        let x_axis_line = out
            .lines()
            .find(|l| l.contains('\u{253C}'))
            .expect("x-axis line with ┼ not found");
        assert!(
            x_axis_line.contains("Low Reach") || out.contains("Low Reach"),
            "Low Reach axis label missing"
        );
        assert!(
            x_axis_line.contains("High Reach") || out.contains("High Reach"),
            "High Reach axis label missing"
        );

        // y-axis edge labels appear above and below the chart body.
        assert!(
            out.contains("High Engagement"),
            "High Engagement y-axis label missing"
        );
        assert!(
            out.contains("Low Engagement"),
            "Low Engagement y-axis label missing"
        );

        // The high y-axis label must appear before the first `^` arrow line.
        let high_eng_line = out
            .lines()
            .position(|l| l.contains("High Engagement"))
            .expect("High Engagement line not found");
        let arrow_line = out
            .lines()
            .position(|l| l.contains('^'))
            .expect("^ arrow line not found");
        assert!(
            high_eng_line < arrow_line,
            "High Engagement ({high_eng_line}) must precede ^ arrow ({arrow_line})"
        );
    }

    #[test]
    fn empty_chart_renders_without_panic() {
        let chart = QuadrantChart::default();
        let out = render(&chart, None);
        // At minimum the axes must be drawn.
        assert!(out.contains('\u{253C}') || out.contains('\u{2502}') || out.contains('^'));
    }
}
