use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use super::navigation::{DetailPayload, Tab};

pub fn render_tab_bar(frame: &mut Frame, area: Rect, tabs: &[Tab], active_index: usize) {
    if tabs.is_empty() {
        return;
    }
    let width_per_tab = (area.width / tabs.len() as u16).max(8);
    let constraints = vec![Constraint::Length(width_per_tab); tabs.len()];
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);
    for (i, tab) in tabs.iter().enumerate() {
        let title = format!(" {} ", tab.label);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if i == active_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });
        frame.render_widget(block, chunks[i]);
    }
}

pub fn render_detail_panel(frame: &mut Frame, area: Rect, payload: &DetailPayload) {
    let lines = match (&payload.title, &payload.body) {
        (None, None) => vec!["No selection yet".into()],
        (Some(title), None) => vec![
            format!("Title: {title}"),
            "Detail panel is waiting for context...".into(),
        ],
        (Some(title), Some(body)) => vec![format!("Title: {title}"), body.clone()],
        (None, Some(body)) => vec!["Title: (unnamed)".into(), body.clone()],
    };
    let paragraph = Paragraph::new(lines.join("\n"))
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
