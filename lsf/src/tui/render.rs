use super::{
    app::AppLifecycleStatus,
    state::{Focus, SortDirection, SortKey, SortState, TuiApp, display_width},
};
use crate::tui::keymap::ResultKeys;
use jiff::Timestamp;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
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
    let status_block = Block::default().borders(Borders::TOP);
    let status_inner = status_block.inner(layout.status);
    frame.render_widget(status_block, layout.status);

    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(24)])
        .split(status_inner);

    let status_left = Paragraph::new(status_bar_line(app));
    frame.render_widget(status_left, status_layout[0]);

    let leader_keys = format_keys(&app.keymap.global.leader);
    let help_keys = format_keys(&app.keymap.leader.help);

    let status_right_line = if app.pending_ctrl_w {
        Line::from(vec![
            Span::styled(
                format!(" {} ", leader_keys),
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" pending ", Style::default().fg(Color::Yellow)),
        ])
    } else {
        Line::from(vec![
            Span::styled(leader_keys, Style::default().fg(Color::DarkGray)),
            Span::styled(" + ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                help_keys,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" help", Style::default().fg(Color::Gray)),
        ])
    };

    let status_right = Paragraph::new(status_right_line).alignment(Alignment::Right);
    frame.render_widget(status_right, status_layout[1]);

    if app.details_popup_open {
        render_popup(frame, app);
    }
    if app.quit_confirm_open {
        render_quit_confirm(frame);
    }
    if app.help_open {
        render_help(frame, app);
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

fn format_keys(specs: &[crate::tui::keymap::KeySpec]) -> String {
    specs
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn popup_details(result: &SearchResultNode, result_keys: &ResultKeys) -> String {
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
        r#"Path: {}
Type: {}
Size: {}
Modified: {}
Created: {}

Press {} to close. Other key bindings:
[{}] Open item (default app)
[{}] Open selected file in editor
[{}] Reveal in Finder
[{}] Copy filename to clipboard
[{}] Copy path to clipboard
"#,
        result.path.display(),
        kind,
        size,
        modified,
        created,
        format_keys(&result_keys.open_details),
        format_keys(&result_keys.open_item),
        format_keys(&result_keys.open_editor),
        format_keys(&result_keys.reveal_in_finder),
        format_keys(&result_keys.copy_filename),
        format_keys(&result_keys.copy_path),
    )
}

fn render_popup(frame: &mut Frame, app: &TuiApp) {
    let Some(result) = app.results.get(app.selected) else {
        return;
    };
    let area = centered_rect(70, 55, frame.area());
    frame.render_widget(Clear, area);
    let popup = Paragraph::new(popup_details(result, &app.keymap.results))
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

fn render_help(frame: &mut Frame, app: &TuiApp) {
    let area = centered_rect(64, 80, frame.area());
    frame.render_widget(Clear, area);

    let k = &app.keymap;
    let leader = format_keys(&k.global.leader);

    let text = format!(
        "\
Keyboard Shortcuts

── Query box ───────────────────────────────
  Type                  Edit search query (live search)
  {:20} Save query to history & search
  {:20} Clear query (or quit if empty)
  {:20} Move cursor left / right
  {:20} Move cursor to start / end
  {:20} Browse query history / move to results
  Tab                  Switch focus to results

── Results table ───────────────────────────
  {:20} Move selection down
  {:20} Move selection up / move to query at top
  Tab                  Switch focus to query
  {:20} Open item details popup
  {:20} Open item (default app)
  {:20} Open selected file in editor
  {:20} Reveal in Finder
  {:20} Copy filename to clipboard
  {:20} Copy path to clipboard
  {:20} Quick Look preview
  {:20} Sort by filename
  {:20} Sort by path
  {:20} Sort by size
  {:20} Sort by modified date
  {:20} Sort by created date

── Global ──────────────────────────────────
  {:10}  {:8} Switch focus to query box
  {:10}  {:8} Switch focus to results table
  {:10}  {:8} Toggle this help panel
  {:20} Switch focus to query box
  {:20} Quit lsf

── Popups ──────────────────────────────────
  {:20} Close popup
  {:20} Open item (default app)
  {:20} Open file in editor (details)
  {:20} Reveal in Finder
  {:20} Copy filename to clipboard
  {:20} Copy path to clipboard
  {:20} Quick Look preview

Press {}, ?, {} or {} to close.",
        // Query Box
        format_keys(&k.query.submit),
        format_keys(&k.query.clear),
        format_keys(&k.query.cursor_left) + " " + &format_keys(&k.query.cursor_right),
        format_keys(&k.query.cursor_home) + " " + &format_keys(&k.query.cursor_end),
        format_keys(&k.query.history_older) + " " + &format_keys(&k.query.history_newer),
        // Results table
        format_keys(&k.results.scroll_down),
        format_keys(&k.results.scroll_up),
        format_keys(&k.results.open_details),
        format_keys(&k.results.open_item),
        format_keys(&k.results.open_editor),
        format_keys(&k.results.reveal_in_finder),
        format_keys(&k.results.copy_filename),
        format_keys(&k.results.copy_path),
        format_keys(&k.results.quick_look),
        format_keys(&k.results.sort_filename),
        format_keys(&k.results.sort_path),
        format_keys(&k.results.sort_size),
        format_keys(&k.results.sort_modified),
        format_keys(&k.results.sort_created),
        // Global
        leader,
        format_keys(&k.leader.focus_query),
        leader,
        format_keys(&k.leader.focus_results),
        leader,
        format_keys(&k.leader.help),
        format_keys(&k.global.focus_query),
        format_keys(&k.global.quit),
        // Popups
        format_keys(&k.results.open_details) + ", Esc",
        format_keys(&k.results.open_item),
        format_keys(&k.results.open_editor),
        format_keys(&k.results.reveal_in_finder),
        format_keys(&k.results.copy_filename),
        format_keys(&k.results.copy_path),
        format_keys(&k.results.quick_look),
        // Footer
        format_keys(&k.global.quit),
        format_keys(&k.leader.help),
        format_keys(&k.results.open_details),
    );

    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Help ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((app.help_scroll, 0));
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
