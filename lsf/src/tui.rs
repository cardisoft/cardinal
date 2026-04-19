use crate::app::{AppLifecycleStatus, AppRuntime, RuntimeStatus, SearchResponse};
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute,
};
use jiff::Timestamp;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use search_cache::SearchResultNode;
use std::{
    cmp::Ordering,
    env,
    io::stdout,
    num::NonZeroU32,
    path::Path,
    process::{Command, ExitStatus},
    time::{Duration, Instant},
};

const SEARCH_DEBOUNCE_MS: u64 = 300;

pub fn run(runtime: &AppRuntime) -> Result<()> {
    run_with_options(runtime, true)
}

pub fn run_with_options(runtime: &AppRuntime, confirm_quit: bool) -> Result<()> {
    execute!(stdout(), EnableMouseCapture)?;
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, runtime, confirm_quit);
    ratatui::restore();
    execute!(stdout(), DisableMouseCapture)?;
    result
}

struct TuiApp {
    query: String,
    cursor: usize,
    history: Vec<String>,
    history_index: Option<usize>,
    history_draft: Option<String>,
    results: Vec<SearchResultNode>,
    total_indexed: usize,
    selected: usize,
    status: String,
    focus: Focus,
    pending_ctrl_w: bool,
    details_popup_open: bool,
    quit_confirm_open: bool,
    confirm_quit: bool,
    sort: Option<SortState>,
    runtime_status: RuntimeStatus,
    last_ready: bool,
    tick: u64,
    /// Debounce for scheduling searches while the user is typing. 
    /// If `Some`, a search is scheduled to run at the specified `Instant`.
    pending_search_at: Option<Instant>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Query,
    Results,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Filename,
    FullPath,
    Size,
    Modified,
    Created,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SortState {
    key: SortKey,
    direction: SortDirection,
}

struct AppLayout {
    query: Rect,
    results: Rect,
    status: Rect,
}

impl TuiApp {
    fn new() -> Self {
        Self {
            query: String::new(),
            cursor: 0,
            history: Vec::new(),
            history_index: None,
            history_draft: None,
            results: Vec::new(),
            total_indexed: 0,
            selected: 0,
            status:
                "Ctrl+W then Up/Down switches focus. Enter in query saves and moves to results."
                    .to_string(),
            focus: Focus::Query,
            pending_ctrl_w: false,
            details_popup_open: false,
            quit_confirm_open: false,
            confirm_quit: true,
            sort: None,
            runtime_status: RuntimeStatus {
                lifecycle: AppLifecycleStatus::Initializing,
                scanned_files: 0,
            },
            last_ready: false,
            tick: 0,
            pending_search_at: None,
        }
    }

    fn apply_search(&mut self, response: SearchResponse) {
        let selected_path = self.selected_result().map(|result| result.path.clone());
        self.results = response.results;
        self.total_indexed = response.total_indexed;
        self.apply_sort();
        self.restore_selection(selected_path.as_deref());
        self.status = format!(
            "{} matches from {} indexed entries",
            self.results.len(),
            self.total_indexed
        );
    }

    fn clear_query(&mut self) {
        self.query.clear();
        self.cursor = 0;
        self.reset_history_navigation();
    }

    fn move_cursor_left(&mut self) {
        self.cursor = previous_char_boundary(&self.query, self.cursor);
    }

    fn move_cursor_right(&mut self) {
        self.cursor = next_char_boundary(&self.query, self.cursor);
    }

    fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor = self.query.len();
    }

    fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.reset_history_navigation();
    }

    fn delete_backwards(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = previous_char_boundary(&self.query, self.cursor);
        self.query.drain(start..self.cursor);
        self.cursor = start;
        self.reset_history_navigation();
        true
    }

    fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        self.pending_ctrl_w = false;
    }

    fn open_popup(&mut self) {
        if !self.results.is_empty() {
            self.details_popup_open = true;
        }
    }

    fn close_popup(&mut self) {
        self.details_popup_open = false;
    }

    fn request_quit(&mut self) -> bool {
        if self.confirm_quit {
            self.quit_confirm_open = true;
            self.status = "Quit requested. Confirm to exit lsf.".to_string();
            false
        } else {
            true
        }
    }

    fn close_quit_confirm(&mut self) {
        self.quit_confirm_open = false;
    }

    fn selected_result(&self) -> Option<&SearchResultNode> {
        self.results.get(self.selected)
    }

    fn set_history(&mut self, history: Vec<String>) {
        self.history = history;
        self.reset_history_navigation();
    }

    fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.history_draft = None;
    }

    fn start_ctrl_w(&mut self) {
        self.pending_ctrl_w = true;
        self.status = "Ctrl+W pending: press Up or Down to switch focus.".to_string();
    }

    fn clear_ctrl_w_pending(&mut self) {
        self.pending_ctrl_w = false;
        self.status = format!(
            "{} matches from {} indexed entries",
            self.results.len(),
            self.total_indexed
        );
    }

    fn update_runtime_status(&mut self, status: RuntimeStatus) {
        let was_ready = self.last_ready;
        self.last_ready = status.lifecycle == AppLifecycleStatus::Ready;
        self.runtime_status = status;
        if !was_ready && self.last_ready {
            self.status = format!(
                "Index ready: {} files scanned.",
                self.runtime_status.scanned_files
            );
        }
    }

    /// Schedule a search to run after a debounce period.
    fn schedule_search(&mut self) {
        self.pending_search_at = Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
    }

    fn search_if_ready(&mut self, runtime: &AppRuntime) {
        self.clear_pending_search();
        if self.runtime_status.lifecycle == AppLifecycleStatus::Ready {
            if self.query.trim().is_empty() {
                self.results.clear();
                self.selected = 0;
                self.total_indexed = self.runtime_status.scanned_files;
                self.status = format!(
                    "Index ready: {} files scanned. Type a query to search.",
                    self.runtime_status.scanned_files
                );
                return;
            }
            match runtime.search(self.query.clone()) {
                Ok(response) => self.apply_search(response),
                Err(err) => {
                    self.results.clear();
                    self.selected = 0;
                    self.total_indexed = self.runtime_status.scanned_files;
                    self.status = format!("Search error: {err}");
                }
            }
        }
    }

    fn clear_pending_search(&mut self) {
        self.pending_search_at = None;
    }

    fn browse_history_older(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(index) => index.saturating_sub(1),
            None => {
                self.history_draft = Some(self.query.clone());
                self.history.len() - 1
            }
        };
        self.apply_history_index(next_index);
    }

    fn browse_history_newer(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 < self.history.len() {
            self.apply_history_index(index + 1);
        } else {
            self.history_index = None;
            self.query = self.history_draft.take().unwrap_or_default();
            self.cursor = self.query.len();
        }
    }

    fn apply_history_index(&mut self, index: usize) {
        self.history_index = Some(index);
        self.query = self.history[index].clone();
        self.cursor = self.query.len();
    }

    fn toggle_sort(&mut self, key: SortKey) {
        let selected_index = self.selected;
        self.sort = match self.sort {
            Some(current) if current.key == key && current.direction == SortDirection::Asc => {
                Some(SortState {
                    key,
                    direction: SortDirection::Desc,
                })
            }
            Some(current) if current.key == key && current.direction == SortDirection::Desc => None,
            _ => Some(SortState {
                key,
                direction: SortDirection::Asc,
            }),
        };
        self.apply_sort();
        self.restore_selection_by_index(selected_index);
        self.status = match self.sort {
            Some(sort) => format!(
                "Sorted by {} ({})",
                sort.key.label(),
                sort.direction.label()
            ),
            None => "Sorting cleared".to_string(),
        };
    }

    fn apply_sort(&mut self) {
        let Some(sort) = self.sort else {
            return;
        };
        self.results
            .sort_by(|left, right| compare_results(left, right, sort));
    }

    fn restore_selection(&mut self, selected_path: Option<&Path>) {
        if self.results.is_empty() {
            self.selected = 0;
            return;
        }
        if let Some(path) = selected_path
            && let Some(index) = self.results.iter().position(|result| result.path == path)
        {
            self.selected = index;
            return;
        }
        self.selected = self.selected.min(self.results.len() - 1);
    }

    fn restore_selection_by_index(&mut self, selected_index: usize) {
        if self.results.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = selected_index.min(self.results.len() - 1);
    }

    fn render(&self, frame: &mut Frame) {
        let layout = layout_for_area(frame.area());

        let query_border = if self.focus == Focus::Query {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let search = Paragraph::new(self.query.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Query")
                .border_style(query_border),
        );
        frame.render_widget(search, layout.query);
        if self.focus == Focus::Query {
            frame.set_cursor_position((
                layout.query.x + 1 + display_width(&self.query[..self.cursor]) as u16,
                layout.query.y + 1,
            ));
        }

        if self.runtime_status.lifecycle == AppLifecycleStatus::Ready {
            let rows: Vec<Row> = self
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

            let mut table_state = TableState::default()
                .with_selected((!self.results.is_empty()).then_some(self.selected));
            let header = Row::new([
                header_label("Filename", SortKey::Filename, self.sort),
                header_label("Path", SortKey::FullPath, self.sort),
                header_label("Size", SortKey::Size, self.sort),
                header_label("Modified", SortKey::Modified, self.sort),
                header_label("Created", SortKey::Created, self.sort),
            ])
            .style(
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            );
            let results_border = if self.focus == Focus::Results {
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
            frame.render_widget(self.indexing_panel(), layout.results);
        }

        let status =
            Paragraph::new(self.status_bar_line()).block(Block::default().borders(Borders::TOP));
        frame.render_widget(status, layout.status);

        if self.details_popup_open {
            render_popup(frame, self);
        }
        if self.quit_confirm_open {
            render_quit_confirm(frame);
        }
    }

    fn indexing_panel(&self) -> Paragraph<'static> {
        let lifecycle = match self.runtime_status.lifecycle {
            AppLifecycleStatus::Initializing => "Initializing index",
            AppLifecycleStatus::Updating => "Updating index",
            AppLifecycleStatus::Ready => "Ready",
        };
        let width = 32usize;
        let pos = (self.tick as usize) % (width + 6);
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
            bar, self.runtime_status.scanned_files
        );
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Indexing"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
    }

    fn status_bar_line(&self) -> Line<'static> {
        let lifecycle = match self.runtime_status.lifecycle {
            AppLifecycleStatus::Initializing => "○ Initializing",
            AppLifecycleStatus::Updating => "◑ Updating",
            AppLifecycleStatus::Ready => "● Ready",
        };
        let results = if self.runtime_status.lifecycle == AppLifecycleStatus::Ready {
            format!("results {}", self.results.len())
        } else {
            "results --".to_string()
        };
        let sort = match self.sort {
            Some(sort) => format!("sort {} {}", sort.key.label(), sort.direction.label()),
            None => "sort off".to_string(),
        };
        Line::from(format!(
            "{} | indexed {} | {} | {} | {}",
            lifecycle, self.runtime_status.scanned_files, results, sort, self.status
        ))
    }
}

impl SortKey {
    fn label(self) -> &'static str {
        match self {
            SortKey::Filename => "filename",
            SortKey::FullPath => "path",
            SortKey::Size => "size",
            SortKey::Modified => "modified",
            SortKey::Created => "created",
        }
    }
}

impl SortDirection {
    fn label(self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }
}

fn run_app(terminal: &mut DefaultTerminal, runtime: &AppRuntime, confirm_quit: bool) -> Result<()> {
    let mut app = TuiApp::new();
    app.confirm_quit = confirm_quit;
    app.set_history(runtime.history()?);
    app.update_runtime_status(runtime.status()?);
    if app.runtime_status.lifecycle == AppLifecycleStatus::Ready {
        app.search_if_ready(runtime);
    }

    loop {
        app.tick = app.tick.wrapping_add(1);
        let latest_status = runtime.status()?;
        let became_ready = app.runtime_status.lifecycle != AppLifecycleStatus::Ready
            && latest_status.lifecycle == AppLifecycleStatus::Ready;
        app.update_runtime_status(latest_status);
        if became_ready {
            app.search_if_ready(runtime);
        }
        if let Some(deadline) = app.pending_search_at
            && Instant::now() >= deadline
        {
            app.search_if_ready(runtime);
        }
        terminal.draw(|frame| app.render(frame))?;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    if app.quit_confirm_open {
                        match key.code {
                            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => break,
                            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                app.close_quit_confirm();
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if app.details_popup_open {
                        match key.code {
                            KeyCode::Enter | KeyCode::Esc => app.close_popup(),
                            KeyCode::Char('q') if key.modifiers.is_empty() => app.close_popup(),
                            KeyCode::Char('v') if key.modifiers.is_empty() => {
                                open_selected_in_editor(terminal, &mut app)?;
                            }
                            KeyCode::Char('1') if key.modifiers.is_empty() => {
                                app.toggle_sort(SortKey::Filename);
                            }
                            KeyCode::Char('2') if key.modifiers.is_empty() => {
                                app.toggle_sort(SortKey::FullPath);
                            }
                            KeyCode::Char('3') if key.modifiers.is_empty() => {
                                app.toggle_sort(SortKey::Size);
                            }
                            KeyCode::Char('4') if key.modifiers.is_empty() => {
                                app.toggle_sort(SortKey::Modified);
                            }
                            KeyCode::Char('5') if key.modifiers.is_empty() => {
                                app.toggle_sort(SortKey::Created);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if app.pending_ctrl_w {
                        match key.code {
                            KeyCode::Up => app.set_focus(Focus::Query),
                            KeyCode::Down => app.set_focus(Focus::Results),
                            _ => {}
                        }
                        app.clear_ctrl_w_pending();
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('q') if key.modifiers.is_empty() => {
                            if app.request_quit() {
                                break;
                            }
                        }
                        KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
                            app.start_ctrl_w();
                        }
                        KeyCode::Esc => {
                            if !app.query.is_empty() && app.focus == Focus::Query {
                                app.clear_query();
                                app.schedule_search();
                            } else if app.focus == Focus::Query {
                                if app.request_quit() {
                                    break;
                                }
                            }
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if !app.query.is_empty() {
                                app.clear_query();
                                app.schedule_search();
                            }
                        }
                        KeyCode::Backspace if app.focus == Focus::Query => {
                            if app.delete_backwards() {
                                app.schedule_search();
                            }
                        }
                        KeyCode::Enter if app.focus == Focus::Query => {
                            let query = app.query.trim().to_string();
                            if !query.is_empty() {
                                app.set_history(runtime.record_history(&query)?);
                                app.status = format!("Saved query to history: {query}");
                            }
                            app.search_if_ready(runtime);
                            app.set_focus(Focus::Results);
                        }
                        KeyCode::Char(ch)
                            if app.focus == Focus::Query
                                && (key.modifiers.is_empty()
                                    || key.modifiers == KeyModifiers::SHIFT) =>
                        {
                            app.insert_char(ch);
                            app.schedule_search();
                        }
                        KeyCode::Left if app.focus == Focus::Query => app.move_cursor_left(),
                        KeyCode::Right if app.focus == Focus::Query => app.move_cursor_right(),
                        KeyCode::Home if app.focus == Focus::Query => app.move_cursor_home(),
                        KeyCode::End if app.focus == Focus::Query => app.move_cursor_end(),
                        KeyCode::Up if app.focus == Focus::Query => {
                            app.browse_history_older();
                            app.schedule_search();
                        }
                        KeyCode::Down if app.focus == Focus::Query => {
                            if app.history_index.is_some() {
                                app.browse_history_newer();
                                app.schedule_search();
                            }
                        }
                        KeyCode::Char('j')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            scroll_results(&mut app, 1);
                        }
                        KeyCode::Char('k')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            scroll_results(&mut app, -1);
                        }
                        KeyCode::Up if app.focus == Focus::Results => {
                            scroll_results(&mut app, -1);
                        }
                        KeyCode::Down if app.focus == Focus::Results => {
                            scroll_results(&mut app, 1);
                        }
                        KeyCode::Enter if app.focus == Focus::Results => app.open_popup(),
                        KeyCode::Char('v')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            open_selected_in_editor(terminal, &mut app)?;
                        }
                        KeyCode::Char('1')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            app.toggle_sort(SortKey::Filename);
                        }
                        KeyCode::Char('2')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            app.toggle_sort(SortKey::FullPath);
                        }
                        KeyCode::Char('3')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            app.toggle_sort(SortKey::Size);
                        }
                        KeyCode::Char('4')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            app.toggle_sort(SortKey::Modified);
                        }
                        KeyCode::Char('5')
                            if app.focus == Focus::Results && key.modifiers.is_empty() =>
                        {
                            app.toggle_sort(SortKey::Created);
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    app.pending_ctrl_w = false;
                    if app.details_popup_open || app.quit_confirm_open {
                        continue;
                    }
                    let size = terminal.size()?;
                    let layout = layout_for_area(Rect::new(0, 0, size.width, size.height));
                    if !layout.results.contains((mouse.column, mouse.row).into()) {
                        continue;
                    }
                    app.set_focus(Focus::Results);
                    match mouse.kind {
                        MouseEventKind::ScrollDown => scroll_results(&mut app, 3),
                        MouseEventKind::ScrollUp => scroll_results(&mut app, -3),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
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

fn popup_details(result: &SearchResultNode) -> String {
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

fn layout_for_area(area: Rect) -> AppLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)])
        .split(vertical[1]);
    AppLayout {
        query: vertical[0],
        results: body[0],
        status: vertical[2],
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

fn previous_char_boundary(text: &str, cursor: usize) -> usize {
    text[..cursor]
        .char_indices()
        .last()
        .map_or(0, |(idx, _)| idx)
}

fn next_char_boundary(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }
    text[cursor..]
        .char_indices()
        .nth(1)
        .map_or(text.len(), |(idx, _)| cursor + idx)
}

fn display_width(text: &str) -> usize {
    text.chars().count()
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

fn compare_results(left: &SearchResultNode, right: &SearchResultNode, sort: SortState) -> Ordering {
    let ordering = match sort.key {
        SortKey::Filename => compare_strings(filename_for_sort(left), filename_for_sort(right)),
        SortKey::FullPath => compare_strings(
            &left.path.display().to_string(),
            &right.path.display().to_string(),
        ),
        SortKey::Size => compare_option_u32(size_for_sort(left), size_for_sort(right))
            .then_with(|| compare_strings(filename_for_sort(left), filename_for_sort(right))),
        SortKey::Modified => compare_option_u32(mtime_for_sort(left), mtime_for_sort(right))
            .then_with(|| compare_strings(filename_for_sort(left), filename_for_sort(right))),
        SortKey::Created => compare_option_u32(ctime_for_sort(left), ctime_for_sort(right))
            .then_with(|| compare_strings(filename_for_sort(left), filename_for_sort(right))),
    };

    match sort.direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

fn compare_strings(left: &str, right: &str) -> Ordering {
    left.to_lowercase()
        .cmp(&right.to_lowercase())
        .then_with(|| left.cmp(right))
}

fn compare_option_u32(left: Option<u32>, right: Option<u32>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn filename_for_sort(result: &SearchResultNode) -> &str {
    result
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}

fn size_for_sort(result: &SearchResultNode) -> Option<u32> {
    result
        .metadata
        .as_ref()
        .as_ref()
        .and_then(|metadata| u32::try_from(metadata.size()).ok())
}

fn mtime_for_sort(result: &SearchResultNode) -> Option<u32> {
    result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.mtime().map(|t| t.get()))
}

fn ctime_for_sort(result: &SearchResultNode) -> Option<u32> {
    result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.ctime().map(|t| t.get()))
}

fn scroll_results(app: &mut TuiApp, delta: isize) {
    if app.results.is_empty() {
        app.selected = 0;
        return;
    }
    let max_index = app.results.len() - 1;
    app.selected = if delta.is_negative() {
        app.selected.saturating_sub(delta.unsigned_abs())
    } else {
        app.selected.saturating_add(delta as usize).min(max_index)
    };
}

fn open_selected_in_editor(terminal: &mut DefaultTerminal, app: &mut TuiApp) -> Result<()> {
    let Some(result) = app.selected_result() else {
        app.status = "No selection to open.".to_string();
        return Ok(());
    };

    let path = result.path.clone();
    ratatui::restore();
    let editor_result = launch_editor(&path);
    *terminal = ratatui::init();

    match editor_result {
        Ok(editor) => {
            app.status = format!("Opened {} with {}", path.display(), editor);
        }
        Err(err) => {
            app.status = format!("Failed to open {}: {}", path.display(), err);
        }
    }

    Ok(())
}

fn launch_editor(path: &Path) -> Result<String> {
    if let Some(editor) = env::var_os("VISUAL") {
        run_shell_editor(&editor.to_string_lossy(), path)?;
        return Ok(editor.to_string_lossy().into_owned());
    }
    if let Some(editor) = env::var_os("EDITOR") {
        run_shell_editor(&editor.to_string_lossy(), path)?;
        return Ok(editor.to_string_lossy().into_owned());
    }

    for editor in ["nvim", "vim"] {
        match Command::new(editor).arg(path).status() {
            Ok(status) if status.success() => return Ok(editor.to_string()),
            Ok(status) => return Err(anyhow::anyhow!(format_exit_status(editor, status))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err.into()),
        }
    }

    Err(anyhow::anyhow!(
        "no editor found; set $VISUAL or $EDITOR, or install nvim/vim"
    ))
}

fn run_shell_editor(editor: &str, path: &Path) -> Result<()> {
    let command = format!("{editor} {}", shell_escape(path));
    let status = Command::new("sh").arg("-lc").arg(command).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(format_exit_status(editor, status)))
    }
}

fn shell_escape(path: &Path) -> String {
    let path = path.display().to_string().replace('\'', "'\\''");
    format!("'{path}'")
}

fn format_exit_status(command: &str, status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("{command} exited with status {code}"),
        None => format!("{command} terminated by signal"),
    }
}
