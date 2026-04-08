use ratatui::style::{Color, Modifier, Style};

use crate::{app::TuiApp, events::TranscriptItemKind};

pub(super) fn prompt() -> Style {
    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
}

pub(super) fn muted() -> Style {
    Style::new().fg(Color::DarkGray)
}

pub(super) fn selected() -> Style {
    Style::new().fg(Color::Black).bg(Color::Gray)
}

pub(super) fn panel_title() -> Style {
    muted().add_modifier(Modifier::BOLD)
}

pub(super) fn composer_border(app: &TuiApp) -> Style {
    if app.busy {
        Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if app.onboarding_prompt.is_some() {
        prompt()
    } else {
        muted()
    }
}

pub(super) fn overlay_border() -> Style {
    Style::new().fg(Color::Gray)
}

pub(super) fn transcript_title(kind: TranscriptItemKind) -> Style {
    match kind {
        TranscriptItemKind::ToolCall | TranscriptItemKind::ToolResult => {
            muted().add_modifier(Modifier::BOLD)
        }
        _ => Style::new().fg(kind.accent()).add_modifier(Modifier::BOLD),
    }
}

pub(super) fn transcript_body(kind: TranscriptItemKind) -> Style {
    match kind {
        TranscriptItemKind::Error => Style::new().fg(kind.accent()),
        TranscriptItemKind::ToolCall | TranscriptItemKind::ToolResult => muted(),
        _ => Style::new(),
    }
}
