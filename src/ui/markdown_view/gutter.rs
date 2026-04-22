use super::visual_rows::line_visual_rows;
use crate::theme::Palette;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

/// Render a slice of text with an absolute-line-number gutter.
///
/// `first_line_number` is the 1-based absolute display line of the slice's
/// first row; `total_doc_lines` is used to size the gutter so width is stable
/// across blocks. `scroll_skip` is the number of visual rows to skip from
/// the top of `text` (matches the `scroll((scroll_skip, 0))` applied to the
/// content paragraph). `wrap` enables ratatui's word wrap on the content
/// pane — Tables pass `false` because their cached layout is already pre-
/// sized to the content width.
#[allow(clippy::too_many_arguments)]
pub fn render_text_with_gutter(
    f: &mut Frame,
    rect: ratatui::layout::Rect,
    text: Text<'static>,
    first_line_number: u32,
    total_doc_lines: u32,
    p: &Palette,
    scroll_skip: u16,
    wrap: bool,
) {
    let num_digits = if total_doc_lines == 0 {
        4
    } else {
        (total_doc_lines.ilog10() + 1).max(4)
    };
    let gutter_width = num_digits + 3;

    let chunks = Layout::horizontal([
        Constraint::Length(crate::cast::u16_from_u32(gutter_width)),
        Constraint::Min(0),
    ])
    .split(rect);

    // The content pane uses `Paragraph::wrap(Wrap { trim: false })`, so a
    // single logical `Line` can occupy multiple visual rows on narrow
    // terminals. The gutter must match that per-row layout: emit the line
    // number on the row where the logical line starts and blank padding on
    // each continuation row, so the number stays visually adjacent to its
    // content.
    //
    // `first_line_number` is the absolute visual row of the first row of
    // `text` after `scroll_skip` rows are skipped. We emit numbers tracking
    // the source-line each logical line came from. Since logical lines map
    // 1:1 to source lines (renderer flushes per source line via the
    // SoftBreak fix in 1.18.3), we number each logical line sequentially —
    // the visual continuation rows of a wrapped line get blank padding so
    // the gutter never gets out of step with the wrapped content.
    let content_width = chunks[1].width;
    let gutter_style = Style::new().fg(p.gutter);
    let mut gutter_lines: Vec<Line<'static>> = Vec::with_capacity(text.lines.len());
    let blank_span = Span::styled(
        format!("{:>width$} | ", "", width = num_digits as usize),
        gutter_style,
    );
    for (i, line) in text.lines.iter().enumerate() {
        gutter_lines.push(Line::from(Span::styled(
            format!(
                "{:>width$} | ",
                first_line_number + crate::cast::u32_sat(i),
                width = num_digits as usize
            ),
            gutter_style,
        )));
        let wraps = line_visual_rows(line, content_width);
        for _ in 1..wraps {
            gutter_lines.push(Line::from(blank_span.clone()));
        }
    }

    let mut gutter_para = Paragraph::new(Text::from(gutter_lines));
    if scroll_skip > 0 {
        gutter_para = gutter_para.scroll((scroll_skip, 0));
    }
    f.render_widget(gutter_para, chunks[0]);

    let mut content_para = Paragraph::new(text);
    if wrap {
        content_para = content_para.wrap(Wrap { trim: false });
    }
    if scroll_skip > 0 {
        content_para = content_para.scroll((scroll_skip, 0));
    }
    f.render_widget(content_para, chunks[1]);
}
