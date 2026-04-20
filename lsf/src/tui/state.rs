use super::{
    app::{AppLifecycleStatus, AppRuntime, RuntimeStatus, SearchResponse},
    keymap::Keymap,
    sort::compare_results,
};
use search_cache::SearchResultNode;
use std::{
    path::Path,
    time::{Duration, Instant},
};

const SEARCH_DEBOUNCE_MS: u64 = 300;

pub(super) struct TuiApp {
    pub query: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub history_draft: Option<String>,
    pub results: Vec<SearchResultNode>,
    pub total_indexed: usize,
    pub selected: usize,
    pub status: String,
    pub focus: Focus,
    pub pending_ctrl_w: bool,
    pub details_popup_open: bool,
    pub quit_confirm_open: bool,
    pub help_open: bool,
    pub help_scroll: u16,
    pub confirm_quit: bool,
    pub sort: Option<SortState>,
    pub runtime_status: RuntimeStatus,
    pub last_ready: bool,
    pub tick: u64,
    /// Debounce for scheduling searches while the user is typing.
    /// If `Some`, a search is scheduled to run at the specified `Instant`.
    pub pending_search_at: Option<Instant>,
    pub keymap: Keymap,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Query,
    Results,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SortKey {
    Filename,
    FullPath,
    Size,
    Modified,
    Created,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct SortState {
    pub key: SortKey,
    pub direction: SortDirection,
}

impl TuiApp {
    pub fn new() -> Self {
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
            help_open: false,
            help_scroll: 0,
            confirm_quit: true,
            sort: None,
            runtime_status: RuntimeStatus {
                lifecycle: AppLifecycleStatus::Initializing,
                scanned_files: 0,
                processed_events: 0,
            },
            last_ready: false,
            tick: 0,
            pending_search_at: None,
            keymap: Keymap::default(),
        }
    }

    /// Apply search results to the app state
    pub fn apply_search_result(&mut self, response: SearchResponse, elapsed: Duration) {
        let selected_path = self.selected_result().map(|result| result.path.clone());
        self.results = response.results;
        self.total_indexed = response.total_indexed;
        self.apply_sort();
        self.restore_selection(selected_path.as_deref());
        self.status = format!(
            "{} matches from {} indexed entries (search took {:?})",
            self.results.len(),
            self.total_indexed,
            elapsed
        );
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
        self.cursor = 0;
        self.reset_history_navigation();
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor = previous_char_boundary(&self.query, self.cursor);
    }

    pub fn move_cursor_right(&mut self) {
        self.cursor = next_char_boundary(&self.query, self.cursor);
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor = self.query.len();
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.reset_history_navigation();
    }

    pub fn delete_backwards(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = previous_char_boundary(&self.query, self.cursor);
        self.query.drain(start..self.cursor);
        self.cursor = start;
        self.reset_history_navigation();
        true
    }

    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        self.pending_ctrl_w = false;
    }

    pub fn open_popup(&mut self) {
        if !self.results.is_empty() {
            self.details_popup_open = true;
        }
    }

    pub fn close_popup(&mut self) {
        self.details_popup_open = false;
    }

    pub fn request_quit(&mut self) -> bool {
        if self.confirm_quit {
            self.quit_confirm_open = true;
            self.status = "Quit requested. Confirm to exit lsf.".to_string();
            false
        } else {
            true
        }
    }

    pub fn close_quit_confirm(&mut self) {
        self.quit_confirm_open = false;
    }

    pub fn selected_result(&self) -> Option<&SearchResultNode> {
        self.results.get(self.selected)
    }

    pub fn set_history(&mut self, history: Vec<String>) {
        self.history = history;
        self.reset_history_navigation();
    }

    fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.history_draft = None;
    }

    pub fn start_ctrl_w(&mut self) {
        self.pending_ctrl_w = true;
        self.status = "Ctrl+W pending: j/k or Up/Down to switch focus, ? for help.".to_string();
    }

    pub fn toggle_help(&mut self) {
        self.help_open = !self.help_open;
        self.help_scroll = 0;
        self.pending_ctrl_w = false;
    }

    pub fn clear_ctrl_w_pending(&mut self) {
        self.pending_ctrl_w = false;
        self.status = format!(
            "{} matches from {} indexed entries",
            self.results.len(),
            self.total_indexed
        );
    }

    pub fn update_runtime_status(&mut self, status: RuntimeStatus) {
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
    pub fn schedule_search(&mut self) {
        self.pending_search_at = Some(Instant::now() + Duration::from_millis(SEARCH_DEBOUNCE_MS));
    }

    pub fn search_if_ready(&mut self, runtime: &AppRuntime) {
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
            let now = Instant::now();

            match runtime.search(self.query.clone()) {
                Ok(response) => {
                    let elapsed = now.elapsed();
                    self.apply_search_result(response, elapsed)
                }
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

    pub fn browse_history_older(&mut self) {
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

    pub fn browse_history_newer(&mut self) {
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

    pub fn toggle_sort(&mut self, key: SortKey) {
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
}

pub(super) fn scroll_results(app: &mut TuiApp, delta: isize) {
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

impl SortKey {
    pub fn label(self) -> &'static str {
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
    pub fn label(self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }
}

pub(super) fn previous_char_boundary(text: &str, cursor: usize) -> usize {
    text[..cursor]
        .char_indices()
        .last()
        .map_or(0, |(idx, _)| idx)
}

pub(super) fn next_char_boundary(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }
    text[cursor..]
        .char_indices()
        .nth(1)
        .map_or(text.len(), |(idx, _)| cursor + idx)
}

pub(super) fn display_width(text: &str) -> usize {
    text.chars().count()
}
