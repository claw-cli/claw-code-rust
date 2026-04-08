use ratatui::{
    layout::Rect,
    text::{Line, Span, Text},
    widgets::Paragraph,
};

use crate::app::TuiApp;

use super::{layout, theme};

pub(super) fn render(app: &TuiApp, inner_width: u16) -> Paragraph<'static> {
    Paragraph::new(Text::from(composer_lines(app, inner_width)))
}

pub(super) fn line_count(app: &TuiApp, inner_width: u16) -> u16 {
    composer_lines(app, inner_width).len() as u16
}

pub(super) fn cursor(app: &TuiApp, area: Rect) -> (u16, u16) {
    let (cursor_x, cursor_y) = if app.onboarding_prompt.is_some() {
        app.input
            .visual_cursor_with_prompt(layout::inner_width(area), app.onboarding_prompt.as_deref())
    } else {
        app.input.visual_cursor(layout::inner_width(area))
    };
    (
        area.x + 1 + cursor_x,
        area.y + 1 + cursor_y.min(layout::inner_height(area).saturating_sub(1)),
    )
}

fn composer_lines(app: &TuiApp, inner_width: u16) -> Vec<Line<'static>> {
    if let Some(prompt) = app.onboarding_prompt.as_deref() {
        return prompt_prefixed_lines(app, inner_width, prompt);
    }

    if app.input.text().is_empty() {
        return vec![Line::from(vec![
            Span::styled("> ", theme::prompt()),
            Span::styled("Type a message or / for commands", theme::muted()),
        ])];
    }

    app.input
        .rendered_lines(inner_width)
        .into_iter()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("> ") {
                Line::from(vec![
                    Span::styled("> ", theme::prompt()),
                    Span::raw(rest.to_string()),
                ])
            } else if let Some(rest) = line.strip_prefix("  ") {
                Line::from(vec![
                    Span::styled("  ", theme::prompt()),
                    Span::raw(rest.to_string()),
                ])
            } else {
                Line::from(line)
            }
        })
        .collect()
}

fn prompt_prefixed_lines(app: &TuiApp, inner_width: u16, prompt: &str) -> Vec<Line<'static>> {
    let prompt_label = format!("{prompt}> ");
    let rendered_input = app
        .input
        .rendered_lines_with_prompt(inner_width, Some(prompt));
    if rendered_input.is_empty() {
        return vec![Line::from(vec![Span::styled(
            prompt_label,
            theme::prompt(),
        )])];
    }

    rendered_input
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                if let Some(rest) = line.strip_prefix(&prompt_label) {
                    Line::from(vec![
                        Span::styled(prompt_label.clone(), theme::prompt()),
                        Span::raw(rest.to_string()),
                    ])
                } else {
                    Line::from(vec![Span::styled(line, theme::prompt())])
                }
            } else {
                Line::from(line)
            }
        })
        .collect()
}
