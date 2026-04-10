use ratatui::{
    layout::Rect,
    text::{Line, Span, Text},
    widgets::Paragraph,
};
use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

pub(super) fn cursor_inline(app: &TuiApp, area: Rect) -> (u16, u16) {
    let layout = inline_status_layout(app, area.width.max(1));
    (
        area.x + layout.cursor_x.min(area.width.saturating_sub(1)),
        area.y,
    )
}

pub(super) fn render_inline_status_bar(app: &TuiApp, width: u16) -> Paragraph<'static> {
    Paragraph::new(Line::from(inline_status_layout(app, width).spans))
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

struct InlineStatusLayout {
    spans: Vec<Span<'static>>,
    cursor_x: u16,
}

fn inline_status_layout(app: &TuiApp, width: u16) -> InlineStatusLayout {
    let width = width.max(1);
    let prompt_prefix = app
        .onboarding_prompt
        .as_deref()
        .map(|prompt| format!("{prompt}> "))
        .unwrap_or_else(|| "› ".to_string());
    let metadata = if app.onboarding_prompt.is_some() {
        None
    } else {
        inline_metadata(app, width)
    };
    let metadata_width = metadata
        .as_ref()
        .map(|value| UnicodeWidthStr::width(value.as_str()))
        .unwrap_or(0);
    let reserved_gap = usize::from(metadata.is_some());
    let input_width = width
        .saturating_sub((metadata_width + reserved_gap) as u16)
        .max(1);
    let input = app.input.inline_line(input_width, &prompt_prefix);
    let rendered_input_width = if app.input.text().is_empty() {
        let placeholder_width =
            usize::from(input_width).saturating_sub(UnicodeWidthStr::width(prompt_prefix.as_str()));
        let placeholder = truncate_width("Type a message or / for commands", placeholder_width);
        let mut spans = vec![
            Span::styled(prompt_prefix.clone(), theme::prompt()),
            Span::styled(placeholder.clone(), theme::muted()),
        ];
        let rendered_width = UnicodeWidthStr::width(prompt_prefix.as_str())
            + UnicodeWidthStr::width(placeholder.as_str());
        if let Some(metadata) = metadata {
            let padding = usize::from(input_width).saturating_sub(rendered_width);
            if padding > 0 {
                spans.push(Span::raw(" ".repeat(padding)));
            }
            spans.push(Span::styled(metadata, theme::muted()));
        }
        return InlineStatusLayout {
            spans,
            cursor_x: input.cursor_x,
        };
    } else {
        UnicodeWidthStr::width(input.text.as_str())
    };
    let mut spans = if let Some(rest) = input.text.strip_prefix(&prompt_prefix) {
        vec![
            Span::styled(prompt_prefix, theme::prompt()),
            Span::raw(rest.to_string()),
        ]
    } else {
        vec![Span::raw(input.text)]
    };

    if let Some(metadata) = metadata {
        let padding = usize::from(input_width).saturating_sub(rendered_input_width);
        spans.push(Span::raw(" ".repeat(padding.max(1))));
        spans.push(Span::styled(metadata, theme::muted()));
    }

    InlineStatusLayout {
        spans,
        cursor_x: input.cursor_x,
    }
}

fn inline_metadata(app: &TuiApp, width: u16) -> Option<String> {
    let state = inline_state_label(app);
    let cwd = compact_cwd(&app.cwd);
    let candidates = [
        format!("{state}  |  {}  |  {cwd}", app.model),
        format!("{}  |  {cwd}", app.model),
        app.model.clone(),
    ];

    candidates
        .into_iter()
        .find(|candidate| UnicodeWidthStr::width(candidate.as_str()) + 14 < usize::from(width))
}

fn inline_state_label(app: &TuiApp) -> String {
    if app.busy {
        let spinner = ["-", "\\", "|", "/"][app.spinner_index % 4];
        format!("{spinner} {}", app.status_message)
    } else if app.status_message.is_empty() || app.status_message == "Ready" {
        "ready".to_string()
    } else {
        app.status_message.clone()
    }
}

fn compact_cwd(cwd: &Path) -> String {
    let parts = cwd
        .components()
        .filter_map(|component| {
            let text = component.as_os_str().to_string_lossy();
            (!text.is_empty()).then_some(text.into_owned())
        })
        .collect::<Vec<_>>();

    match parts.as_slice() {
        [] => ".".to_string(),
        [single] => single.clone(),
        [.., parent, leaf] => format!("{parent}\\{leaf}"),
    }
}

fn truncate_width(value: &str, max_width: usize) -> String {
    let mut rendered = String::new();
    let mut width = 0usize;

    for ch in value.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
        if width + ch_width > max_width {
            break;
        }
        rendered.push(ch);
        width += ch_width;
    }

    rendered
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
