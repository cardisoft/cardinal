use super::{
    actions::{
        copy_selected_filename, copy_selected_path, open_selected_in_editor, open_selected_item,
        quick_look_selected, reveal_in_finder,
    },
    app::{AppLifecycleStatus, AppRuntime},
    keymap::match_key,
    render::{layout_for_area, render},
    state::{Focus, SortKey, TuiApp, scroll_results},
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::{DefaultTerminal, layout::Rect};
use std::time::{Duration, Instant};

pub(super) fn run_app(
    terminal: &mut DefaultTerminal,
    runtime: &AppRuntime,
    confirm_quit: bool,
    keymap: super::keymap::Keymap,
) -> Result<()> {
    let mut app = TuiApp::new();
    app.confirm_quit = confirm_quit;
    app.keymap = keymap;
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
        terminal.draw(|frame| render(&app, frame))?;
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
                        let km = &app.keymap;
                        if key.code == KeyCode::Enter
                            || key.code == KeyCode::Esc
                            || match_key(&km.global.quit, key.code, key.modifiers)
                        {
                            app.close_popup();
                        } else if match_key(&km.results.open_editor, key.code, key.modifiers) {
                            open_selected_in_editor(terminal, &mut app)?;
                        } else if match_key(&km.results.open_item, key.code, key.modifiers) {
                            open_selected_item(&mut app);
                        } else if match_key(&km.results.reveal_in_finder, key.code, key.modifiers) {
                            reveal_in_finder(&mut app);
                        } else if match_key(&km.results.copy_filename, key.code, key.modifiers) {
                            copy_selected_filename(&mut app);
                        } else if match_key(&km.results.copy_path, key.code, key.modifiers) {
                            copy_selected_path(&mut app);
                        } else if match_key(&km.results.quick_look, key.code, key.modifiers) {
                            quick_look_selected(&mut app);
                        } else if match_key(&km.results.sort_filename, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Filename);
                        } else if match_key(&km.results.sort_path, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::FullPath);
                        } else if match_key(&km.results.sort_size, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Size);
                        } else if match_key(&km.results.sort_modified, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Modified);
                        } else if match_key(&km.results.sort_created, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Created);
                        }
                        continue;
                    }

                    if app.help_open {
                        let km = &app.keymap;
                        if match_key(&km.leader.help, key.code, key.modifiers)
                            || match_key(&km.global.quit, key.code, key.modifiers)
                            || key.code == KeyCode::Esc
                            || key.code == KeyCode::Enter
                        {
                            app.help_open = false;
                            app.help_scroll = 0;
                        } else if match_key(&km.results.scroll_down, key.code, key.modifiers) {
                            app.help_scroll = app.help_scroll.saturating_add(1);
                        } else if match_key(&km.results.scroll_up, key.code, key.modifiers) {
                            app.help_scroll = app.help_scroll.saturating_sub(1);
                        }
                        continue;
                    }

                    if app.pending_ctrl_w {
                        let km = &app.keymap;
                        if match_key(&km.leader.focus_query, key.code, key.modifiers) {
                            app.set_focus(Focus::Query);
                        } else if match_key(&km.leader.focus_results, key.code, key.modifiers) {
                            app.set_focus(Focus::Results);
                        } else if match_key(&km.leader.help, key.code, key.modifiers) {
                            app.toggle_help();
                        }
                        app.clear_ctrl_w_pending();
                        continue;
                    }

                    // --- global leader ---
                    if match_key(&app.keymap.global.leader, key.code, key.modifiers) {
                        app.start_ctrl_w();
                        continue;
                    }

                    if app.focus == Focus::Query {
                        if match_key(&app.keymap.query.clear, key.code, key.modifiers) {
                            if !app.query.is_empty() {
                                app.clear_query();
                                app.schedule_search();
                            } else if app.request_quit() {
                                break;
                            }
                        } else if key.code == KeyCode::Backspace {
                            if app.delete_backwards() {
                                app.schedule_search();
                            }
                        } else if match_key(&app.keymap.query.submit, key.code, key.modifiers) {
                            let query = app.query.trim().to_string();
                            if !query.is_empty() {
                                app.set_history(runtime.record_history(&query)?);
                                app.status = format!("Saved query to history: {query}");
                            }
                            app.search_if_ready(runtime);
                            app.set_focus(Focus::Results);
                        } else if match_key(
                            &app.keymap.query.history_older,
                            key.code,
                            key.modifiers,
                        ) {
                            app.browse_history_older();
                            app.schedule_search();
                        } else if match_key(
                            &app.keymap.query.history_newer,
                            key.code,
                            key.modifiers,
                        ) {
                            if app.history_index.is_some() {
                                app.browse_history_newer();
                                app.schedule_search();
                            }
                        } else if match_key(&app.keymap.query.cursor_left, key.code, key.modifiers)
                        {
                            app.move_cursor_left();
                        } else if match_key(&app.keymap.query.cursor_right, key.code, key.modifiers)
                        {
                            app.move_cursor_right();
                        } else if match_key(&app.keymap.query.cursor_home, key.code, key.modifiers)
                        {
                            app.move_cursor_home();
                        } else if match_key(&app.keymap.query.cursor_end, key.code, key.modifiers) {
                            app.move_cursor_end();
                        } else if let KeyCode::Char(ch) = key.code
                            && (key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT)
                        {
                            app.insert_char(ch);
                            app.schedule_search();
                        }
                    } else {
                        // Focus::Results
                        let km = &app.keymap;
                        if match_key(&km.global.quit, key.code, key.modifiers) {
                            if app.request_quit() {
                                break;
                            }
                        } else if match_key(&km.results.focus_out, key.code, key.modifiers) {
                            app.set_focus(Focus::Query);
                        } else if match_key(&km.results.scroll_down, key.code, key.modifiers) {
                            scroll_results(&mut app, 1);
                        } else if match_key(&km.results.scroll_up, key.code, key.modifiers) {
                            scroll_results(&mut app, -1);
                        } else if match_key(&km.results.open_details, key.code, key.modifiers) {
                            app.open_popup();
                        } else if match_key(&km.results.open_editor, key.code, key.modifiers) {
                            open_selected_in_editor(terminal, &mut app)?;
                        } else if match_key(&km.results.open_item, key.code, key.modifiers) {
                            open_selected_item(&mut app);
                        } else if match_key(&km.results.reveal_in_finder, key.code, key.modifiers) {
                            reveal_in_finder(&mut app);
                        } else if match_key(&km.results.copy_filename, key.code, key.modifiers) {
                            copy_selected_filename(&mut app);
                        } else if match_key(&km.results.copy_path, key.code, key.modifiers) {
                            copy_selected_path(&mut app);
                        } else if match_key(&km.results.quick_look, key.code, key.modifiers) {
                            quick_look_selected(&mut app);
                        } else if match_key(&km.results.sort_filename, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Filename);
                        } else if match_key(&km.results.sort_path, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::FullPath);
                        } else if match_key(&km.results.sort_size, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Size);
                        } else if match_key(&km.results.sort_modified, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Modified);
                        } else if match_key(&km.results.sort_created, key.code, key.modifiers) {
                            app.toggle_sort(SortKey::Created);
                        }
                    }
                }
                Event::Mouse(mouse) if app.focus == Focus::Results => {
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
