use crate::api::Batch;
use crate::app::App;
use chrono::{Local, TimeZone};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::Frame;
use std::collections::BTreeMap;

pub fn status_color(status: &str) -> Color {
    match status {
        "validating" => Color::Yellow,
        "in_progress" => Color::Cyan,
        "finalizing" => Color::Magenta,
        "completed" => Color::Green,
        "failed" => Color::Red,
        "expired" => Color::DarkGray,
        "cancelling" => Color::Yellow,
        "cancelled" => Color::Gray,
        _ => Color::White,
    }
}

fn fmt_ts(ts: Option<i64>) -> String {
    match ts {
        Some(ts) => Local
            .timestamp_opt(ts, 0)
            .single()
            .map(|dt| dt.format("%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string()),
        None => "-".to_string(),
    }
}

fn short_id(id: &str) -> String {
    if id.len() > 22 {
        format!("{}…{}", &id[..14], &id[id.len() - 4..])
    } else {
        id.to_string()
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(size);

    draw_header(frame, app, chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    draw_table(frame, app, body[0]);
    draw_details(frame, app, body[1]);

    draw_footer(frame, app, chunks[2]);

    if app.show_help {
        draw_help_popup(frame, size);
    } else if app.confirm_cancel {
        draw_confirm_popup(frame, app, size);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for b in &app.batches {
        *counts.entry(b.status.as_str()).or_insert(0) += 1;
    }

    let mut spans = vec![
        Span::styled(
            " OpenAI Batch Monitor ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];
    if counts.is_empty() && !app.loading {
        spans.push(Span::styled(
            "no batches",
            Style::default().fg(Color::DarkGray),
        ));
    }
    for (status, count) in &counts {
        spans.push(Span::styled(
            format!(" {status}:{count} "),
            Style::default().fg(status_color(status)),
        ));
    }

    let refresh_info = match app.last_refresh {
        Some(t) => format!(
            "updated {}  (next in {}s)",
            t.format("%H:%M:%S"),
            app.seconds_until_refresh
        ),
        None => "loading…".to_string(),
    };

    let top_line = Line::from(spans);
    let bottom_line = Line::from(vec![Span::styled(
        refresh_info,
        Style::default().fg(Color::DarkGray),
    )]);

    let block = Block::default().borders(Borders::BOTTOM);
    frame.render_widget(
        Paragraph::new(vec![top_line, bottom_line]).block(block),
        area,
    );
}

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Status"),
        Cell::from("Endpoint"),
        Cell::from("Created"),
        Cell::from("Progress"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .batches
        .iter()
        .map(|b| {
            let progress = match &b.request_counts {
                Some(rc) if rc.total > 0 => {
                    format!("{}/{} ({} failed)", rc.completed, rc.total, rc.failed)
                }
                Some(_) => "0/0".to_string(),
                None => "-".to_string(),
            };
            Row::new(vec![
                Cell::from(short_id(&b.id)),
                Cell::from(Span::styled(
                    b.status.clone(),
                    Style::default().fg(status_color(&b.status)),
                )),
                Cell::from(b.endpoint.clone()),
                Cell::from(fmt_ts(Some(b.created_at))),
                Cell::from(progress),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(21),
        Constraint::Length(12),
        Constraint::Length(22),
        Constraint::Length(14),
        Constraint::Min(16),
    ];

    let title = if app.loading {
        " Batches (loading…) "
    } else {
        " Batches "
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn detail_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<16}"), Style::default().fg(Color::DarkGray)),
        Span::raw(value),
    ])
}

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Details ");
    let Some(batch) = app.selected_batch() else {
        frame.render_widget(Paragraph::new("No batch selected").block(block), area);
        return;
    };

    let mut lines = vec![
        detail_line("ID", batch.id.clone()),
        detail_line("Status", batch.status.clone()),
        detail_line("Endpoint", batch.endpoint.clone()),
        detail_line(
            "Window",
            batch
                .completion_window
                .clone()
                .unwrap_or_else(|| "-".into()),
        ),
        Line::from(""),
        detail_line("Created", fmt_ts(Some(batch.created_at))),
        detail_line("In progress", fmt_ts(batch.in_progress_at)),
        detail_line("Finalizing", fmt_ts(batch.finalizing_at)),
        detail_line("Completed", fmt_ts(batch.completed_at)),
        detail_line("Failed", fmt_ts(batch.failed_at)),
        detail_line("Expired", fmt_ts(batch.expired_at)),
        detail_line("Cancelling", fmt_ts(batch.cancelling_at)),
        detail_line("Cancelled", fmt_ts(batch.cancelled_at)),
        detail_line("Expires at", fmt_ts(batch.expires_at)),
        Line::from(""),
    ];

    if let Some(rc) = &batch.request_counts {
        lines.push(detail_line("Total", rc.total.to_string()));
        lines.push(detail_line("Completed", rc.completed.to_string()));
        lines.push(detail_line("Failed", rc.failed.to_string()));
        if rc.total > 0 {
            let pct = (rc.completed as f64 / rc.total as f64) * 100.0;
            lines.push(detail_line("Progress", format!("{pct:.1}%")));
        }
        lines.push(Line::from(""));
    }

    lines.push(detail_line(
        "Input file",
        batch.input_file_id.clone().unwrap_or_else(|| "-".into()),
    ));
    lines.push(detail_line(
        "Output file",
        batch.output_file_id.clone().unwrap_or_else(|| "-".into()),
    ));
    lines.push(detail_line(
        "Error file",
        batch.error_file_id.clone().unwrap_or_else(|| "-".into()),
    ));

    if let Some(errors) = &batch.errors {
        if !errors.data.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Errors:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for e in &errors.data {
                lines.push(Line::from(Span::styled(
                    format!(
                        "  [{}] {}",
                        e.code.clone().unwrap_or_default(),
                        e.message.clone().unwrap_or_default()
                    ),
                    Style::default().fg(Color::Red),
                )));
            }
        }
    }

    if let Some(meta) = &batch.metadata {
        if meta.is_object() && !meta.as_object().unwrap().is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Metadata:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for (k, v) in meta.as_object().unwrap() {
                lines.push(detail_line(k, v.to_string()));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = if let Some(err) = &app.error {
        Line::from(Span::styled(
            format!(" error: {err}"),
            Style::default().fg(Color::Red),
        ))
    } else if let Some((msg, _)) = &app.status_message {
        Line::from(Span::styled(
            format!(" {msg}"),
            Style::default().fg(Color::Green),
        ))
    } else {
        Line::from(Span::styled(
            " j/k or ↑/↓ select · r refresh · c cancel · s sort · ? help · q quit",
            Style::default().fg(Color::DarkGray),
        ))
    };
    frame.render_widget(Paragraph::new(text), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(60, 60, area);
    frame.render_widget(Clear, popup);
    let text = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("j / ↓        select next"),
        Line::from("k / ↑        select previous"),
        Line::from("g / Home     select first"),
        Line::from("G / End      select last"),
        Line::from("r            refresh now"),
        Line::from("c            cancel selected batch"),
        Line::from("s            toggle sort (created / status)"),
        Line::from("?            toggle this help"),
        Line::from("q / Esc      quit"),
        Line::from(""),
        Line::from("Cancellable statuses: validating, in_progress, finalizing"),
        Line::from(""),
        Line::from(Span::styled(
            "press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .style(Style::default().bg(Color::Black));
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_confirm_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup);
    let id = app
        .selected_batch()
        .map(|b: &Batch| b.id.clone())
        .unwrap_or_default();
    let text = vec![
        Line::from(Span::styled(
            "Cancel this batch?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(id),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" confirm    "),
            Span::styled(
                "n/Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" abort"),
        ]),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm ")
        .style(Style::default().bg(Color::Black));
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center),
        popup,
    );
}
