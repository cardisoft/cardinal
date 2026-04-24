use super::{
    app::AppLifecycleStatus,
    state::{Focus, SortDirection, SortKey, SortState, TuiApp, display_width},
};
use jiff::Timestamp;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use search_cache::SearchResultNode;
use std::{num::NonZeroU32, time::Duration};

pub(super) struct AppLayout {
    pub query: Rect,
    pub results: Rect,
    pub status: Rect,
}

pub(super) fn render(app: &TuiApp, frame: &mut Frame) {
    let layout = layout_for_area(frame.area());

    // 1. query input box
    let query_border = if app.focus == Focus::Query {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let search = Paragraph::new(app.query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Query")
            .border_style(query_border),
    );
    frame.render_widget(search, layout.query);
    if app.focus == Focus::Query {
        frame.set_cursor_position((
            layout.query.x + 1 + display_width(&app.query[..app.cursor]) as u16,
            layout.query.y + 1,
        ));
    }

    // 2. result table or indexing panel (the result is not ready yet)
    if app.runtime_status.lifecycle == AppLifecycleStatus::Ready {
        let rows: Vec<Row> = app
            .results
            .iter()
            .map(|result| {
                let columns = result_columns(result);
                Row::new([
                    Cell::from(columns.filename),
                    Cell::from(columns.directory),
                    Cell::from(columns.size),
                    Cell::from(columns.modified),
                    Cell::from(columns.created),
                ])
            })
            .collect();

        let mut table_state =
            TableState::default().with_selected((!app.results.is_empty()).then_some(app.selected));
        let header = Row::new([
            header_label("Filename", SortKey::Filename, app.sort),
            header_label("Path", SortKey::FullPath, app.sort),
            header_label("Size", SortKey::Size, app.sort),
            header_label("Modified", SortKey::Modified, app.sort),
            header_label("Created", SortKey::Created, app.sort),
        ])
        .style(
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        );
        let results_border = if app.focus == Focus::Results {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(22),
                Constraint::Percentage(34),
                Constraint::Length(10),
                Constraint::Length(21),
                Constraint::Length(21),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Results")
                .border_style(results_border),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(25, 52, 77))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
        frame.render_stateful_widget(table, layout.results, &mut table_state);
    } else {
        frame.render_widget(indexing_panel(app), layout.results);
    }

    // 3. status bar
    let status = Paragraph::new(status_bar_line(app)).block(Block::default().borders(Borders::TOP));
    frame.render_widget(status, layout.status);

    if app.details_popup_open {
        render_popup(frame, app);
    }
    if app.quit_confirm_open {
        render_quit_confirm(frame);
    }
    if app.help_open {
        render_help(frame, app.help_scroll);
    }
}

/// Render the indexing panel with a progress bar and status text.
fn indexing_panel(app: &TuiApp) -> Paragraph<'static> {
    let lifecycle = match app.runtime_status.lifecycle {
        AppLifecycleStatus::Initializing => "Initializing index",
        AppLifecycleStatus::Updating => "Updating index",
        _ => unreachable!(),
    };
    let width = 32usize;
    let pos = (app.tick as usize) % (width + 6);
    let bar: String = (0..width)
        .map(|idx| {
            if idx >= pos.saturating_sub(5) && idx <= pos.min(width.saturating_sub(1)) {
                '='
            } else {
                ' '
            }
        })
        .collect();
    let text = format!(
        "{lifecycle}\n\n[{}]\n\nScanned files: {}\n\nYou can type a query now; results will appear when indexing is ready.",
        bar, app.runtime_status.scanned_files
    );
    Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Indexing"))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
}

fn status_bar_line(app: &TuiApp) -> Line<'static> {
    let lifecycle = match app.runtime_status.lifecycle {
        AppLifecycleStatus::Initializing => "○ Initializing",
        AppLifecycleStatus::Updating => "◑ Updating",
        AppLifecycleStatus::Ready => "● Ready",
    };
    let results = if app.runtime_status.lifecycle == AppLifecycleStatus::Ready {
        format!("results {}", app.results.len())
    } else {
        "results --".to_string()
    };
    let sort = match app.sort {
        Some(sort) => format!("sort {} {}", sort.key.label(), sort.direction.label()),
        None => "sort off".to_string(),
    };
    let events = app.runtime_status.processed_events;
    Line::from(format!(
        "{} | indexed {} | events {} | {} | {} | {}",
        lifecycle, app.runtime_status.scanned_files, events, results, sort, app.status
    ))
}

struct ResultColumns {
    filename: String,
    directory: String,
    size: String,
    modified: String,
    created: String,
}

fn result_columns(result: &SearchResultNode) -> ResultColumns {
    let full_path = result.path.display().to_string();
    let filename = result
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| full_path.clone());
    let directory = result
        .path
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| "/".to_string());
    let metadata = result.metadata.as_ref();
    let size = metadata
        .as_ref()
        .map(|metadata| format_size(metadata.size()))
        .unwrap_or_else(|| "—".to_string());
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.mtime())
        .map(format_unix_timestamp)
        .unwrap_or_else(|| "—".to_string());
    let created = metadata
        .as_ref()
        .and_then(|metadata| metadata.ctime())
        .map(format_unix_timestamp)
        .unwrap_or_else(|| "—".to_string());

    ResultColumns {
        filename,
        directory,
        size,
        modified,
        created,
    }
}

pub(super) fn popup_details(result: &SearchResultNode) -> String {
    let kind = result
        .metadata
        .as_ref()
        .as_ref()
        .map(|metadata| format_file_type(metadata.r#type()))
        .unwrap_or_else(|| "unknown".to_string());
    let size = result
        .metadata
        .as_ref()
        .as_ref()
        .map(|metadata| format_size(metadata.size()))
        .unwrap_or_else(|| "n/a".to_string());
    let modified = result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.mtime())
        .map(format_unix_timestamp)
        .unwrap_or_else(|| "n/a".to_string());
    let created = result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.ctime())
        .map(format_unix_timestamp)
        .unwrap_or_else(|| "n/a".to_string());

    format!(
        "Path: {}\nType: {}\nSize: {}\nModified: {}\nCreated: {}\n\nPress Enter or Esc to close.",
        result.path.display(),
        kind,
        size,
        modified,
        created
    )
}

fn render_popup(frame: &mut Frame, app: &TuiApp) {
    let Some(result) = app.results.get(app.selected) else {
        return;
    };
    let area = centered_rect(70, 55, frame.area());
    frame.render_widget(Clear, area);
    let popup = Paragraph::new(popup_details(result))
        .block(
            Block::default()
                .title("Item Details")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(popup, area);
}

fn render_help(frame: &mut Frame, scroll: u16) {
    let area = centered_rect(64, 80, frame.area());
    frame.render_widget(Clear, area);
    let text = "\
Keyboard Shortcuts

── Query box ───────────────────────────────
  Type          Edit search query (live search)
  Enter         Save query to history & search
  Esc           Clear query (or quit if empty)
  Ctrl+U        Clear query
  ←  →          Move cursor left / right
  Home / End    Move cursor to start / end
  ↑  ↓          Browse query history

── Results table ───────────────────────────
  j / ↓         Move selection down
  k / ↑         Move selection up
  Enter         Open item details popup
  o             Open item (default app)
  v             Open selected file in editor
  r             Reveal in Finder
  y             Copy filename to clipboard
  c             Copy path to clipboard
  Space         Quick Look preview
  1             Sort by filename
  2             Sort by path
  3             Sort by size
  4             Sort by modified date
  5             Sort by created date

── Global ──────────────────────────────────
  Ctrl+W  j/↑   Switch focus to query box
  Ctrl+W  k/↓   Switch focus to results table
  Ctrl+W  ?     Toggle this help panel
  q             Quit lsf

── Popups ──────────────────────────────────
  Esc / Enter   Close popup
  q             Close details popup
  o             Open item (default app)
  v             Open file in editor (details)
  r             Reveal in Finder
  y             Copy filename to clipboard
  c             Copy path to clipboard
  Space         Quick Look preview

Press q, ?, Esc or Enter to close.";
    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Help ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((scroll, 0));
    frame.render_widget(popup, area);
}

fn render_quit_confirm(frame: &mut Frame) {
    let area = centered_rect(44, 24, frame.area());
    frame.render_widget(Clear, area);
    let popup = Paragraph::new("Quit lsf?\n\nPress Enter or y to quit.\nPress Esc or n to stay.")
        .block(
            Block::default()
                .title("Confirm Quit")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    frame.render_widget(popup, area);
}

fn centered_rect(horizontal_percent: u16, vertical_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - vertical_percent) / 2),
        Constraint::Percentage(vertical_percent),
        Constraint::Percentage((100 - vertical_percent) / 2),
    ])
    .split(area);
    let horizontal = Layout::horizontal([
        Constraint::Percentage((100 - horizontal_percent) / 2),
        Constraint::Percentage(horizontal_percent),
        Constraint::Percentage((100 - horizontal_percent) / 2),
    ])
    .split(vertical[1]);
    horizontal[1]
}

/// Layout: 3 vertical sections - query, results, status
pub(super) fn layout_for_area(area: Rect) -> AppLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);
    AppLayout {
        query: vertical[0],
        results: vertical[1],
        status: vertical[2],
    }
}

fn header_label(label: &str, key: SortKey, sort: Option<SortState>) -> String {
    match sort {
        Some(active) if active.key == key => {
            let arrow = match active.direction {
                SortDirection::Asc => "↑",
                SortDirection::Desc => "↓",
            };
            format!("{label} {arrow}")
        }
        _ => label.to_string(),
    }
}

fn format_file_type(file_type: fswalk::NodeFileType) -> String {
    match file_type {
        fswalk::NodeFileType::File => "file".to_string(),
        fswalk::NodeFileType::Dir => "dir".to_string(),
        fswalk::NodeFileType::Symlink => "symlink".to_string(),
        fswalk::NodeFileType::Unknown => "unknown".to_string(),
    }
}

fn format_size(size: i64) -> String {
    if size < 0 {
        return "-".to_string();
    }

    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", size, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_unix_timestamp(timestamp: NonZeroU32) -> String {
    Timestamp::from_second(timestamp.get() as i64)
        .map(|ts| ts.to_string())
        .unwrap_or_else(|_| {
            format!(
                "unix:{}",
                Duration::from_secs(timestamp.get() as u64).as_secs()
            )
        })
}
