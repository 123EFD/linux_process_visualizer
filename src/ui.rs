use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

use crate::app::{App, SortColumn};
use crate::{SystemOverview, format_kb, format_uptime};

/// Returns a human-readable display string for the current active sort column.
pub fn sort_label(sort_by: SortColumn) -> &'static str {
    match sort_by {
        SortColumn::Cpu => "CPU% (desc)",
        SortColumn::Memory => "MEM (desc)",
        SortColumn::Pid => "PID (asc)",
        SortColumn::Name => "NAME (asc)",
    }
}

pub fn format_header_lines(overview: &SystemOverview) -> (String, String) {
    let line1 = format!(
        "Uptime: {}  |  Load Avg: {:.2}, {:.2}, {:.2}",
        format_uptime(overview.uptime_seconds),
        overview.load.one,
        overview.load.five,
        overview.load.fifteen
    );
    let line2 = format!(
        "Memory: {} / {} ({:.1}%)",
        format_kb(overview.memory.used_kb()),
        format_kb(overview.memory.total_kb),
        overview.memory.used_percent()
    );
    (line1, line2)
}

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header
            Constraint::Min(10),   // Table
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_table(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);
}

pub fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(ref overview) = app.overview {
        let (line1, line2) = format_header_lines(overview);
        vec![Line::from(line1), Line::from(line2)]
    } else {
        vec![Line::from("Loading system metrics...")]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" System Overview ")
        .style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

pub fn render_table(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells = [
        if app.sort_by == SortColumn::Pid {
            "PID ▲"
        } else {
            "PID"
        },
        if app.sort_by == SortColumn::Name {
            "NAME ▲"
        } else {
            "NAME"
        },
        "STATE",
        if app.sort_by == SortColumn::Cpu {
            "CPU% ▼"
        } else {
            "CPU%"
        },
        if app.sort_by == SortColumn::Memory {
            "MEM ▼"
        } else {
            "MEM"
        },
        "THREADS",
        "CMDLINE",
    ]
    .into_iter()
    .map(|h| {
        Cell::from(h).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1)
        .bottom_margin(1);

    let rows : Vec<Row> = app
        .processes
        .iter()
        .map(|p| {
            Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(p.name.clone()),
                Cell::from(format!("{:?}", p.state)),
                Cell::from(format!("{:.1}%", p.cpu_percent)),
                Cell::from(format_kb(p.rss_kb)),
                Cell::from(p.threads.to_string()),
                Cell::from(p.cmdline.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(8),
        Constraint::Min(20),
    ];

    let table = Table::new(rows,widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Processes ({}) ",
            app.processes.len()
        )))
        .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    let mut state = TableState::default();
    if !app.processes.is_empty() {
        state.select(Some(app.selected_index));
    }
    frame.render_stateful_widget(table, area, &mut state);

}

/// Step 3.5: Renders the Footer block with keyboard shortcuts and sort status.
pub fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let footer_text = Line::from(vec![
        Span::styled(
            " [q] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Quit  "),
        Span::styled(
            " [s] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("Sort: {}  ", sort_label(app.sort_by))),
        Span::styled(
            " [↑/k] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Up  "),
        Span::styled(
            " [↓/j] ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Down"),
    ]);

    let footer_block = Block::default().borders(Borders::ALL).title(" Controls ");

    let paragraph = Paragraph::new(footer_text).block(footer_block);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoadAvg, MemInfo};

    #[test]
    fn test_sort_label() {
        assert_eq!(sort_label(SortColumn::Cpu), "CPU% (desc)");
        assert_eq!(sort_label(SortColumn::Memory), "MEM (desc)");
        assert_eq!(sort_label(SortColumn::Pid), "PID (asc)");
        assert_eq!(sort_label(SortColumn::Name), "NAME (asc)");
    }

    #[test]
    fn test_format_header_lines() {
        let overview = SystemOverview {
            uptime_seconds: 90061.0, // 1d 1h 1m 1s
            load: LoadAvg {
                one: 0.50,
                five: 0.75,
                fifteen: 1.00,
            },
            memory: MemInfo {
                total_kb: 16_000_000,
                available_kb: 8_000_000,
            },
        };

        let (line1, line2) = format_header_lines(&overview);
        assert!(line1.contains("Uptime: 1d 1h 1m 1s"));
        assert!(line1.contains("Load Avg: 0.50, 0.75, 1.00"));
        assert!(line2.contains("50.0%"));
    }
}
