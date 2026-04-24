use super::state::TuiApp;
use anyhow::Result;
use ratatui::DefaultTerminal;
use std::{
    env,
    path::Path,
    process::{Command, ExitStatus},
};

pub(super) fn open_selected_in_editor(
    terminal: &mut DefaultTerminal,
    app: &mut TuiApp,
) -> Result<()> {
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

pub(super) fn open_selected_item(app: &mut TuiApp) {
    let Some(result) = app.selected_result() else {
        app.status = "No selection to open.".to_string();
        return;
    };
    let path = result.path.clone();
    match Command::new("open").arg(&path).status() {
        Ok(status) if status.success() => {
            app.status = format!("Opened {}", path.display());
        }
        Ok(status) => {
            app.status = format!(
                "Failed to open {}: {}",
                path.display(),
                format_exit_status("open", status)
            );
        }
        Err(err) => {
            app.status = format!("Failed to open {}: {err}", path.display());
        }
    }
}

pub(super) fn reveal_in_finder(app: &mut TuiApp) {
    let Some(result) = app.selected_result() else {
        app.status = "No selection to reveal.".to_string();
        return;
    };
    let path = result.path.clone();
    match Command::new("open").arg("-R").arg(&path).status() {
        Ok(status) if status.success() => {
            app.status = format!("Revealed {}", path.display());
        }
        Ok(status) => {
            app.status = format!(
                "Failed to reveal {}: {}",
                path.display(),
                format_exit_status("open -R", status)
            );
        }
        Err(err) => {
            app.status = format!("Failed to reveal {}: {err}", path.display());
        }
    }
}

pub(super) fn copy_selected_filename(app: &mut TuiApp) {
    let Some(result) = app.selected_result() else {
        app.status = "No selection to copy.".to_string();
        return;
    };
    let filename = result
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| result.path.display().to_string());
    copy_to_clipboard(&filename, app);
    app.status = format!("Copied filename: {filename}");
}

pub(super) fn copy_selected_path(app: &mut TuiApp) {
    let Some(result) = app.selected_result() else {
        app.status = "No selection to copy.".to_string();
        return;
    };
    let path = result.path.display().to_string();
    copy_to_clipboard(&path, app);
    app.status = format!("Copied path: {path}");
}

fn copy_to_clipboard(text: &str, app: &mut TuiApp) {
    use std::io::Write;
    let mut child = match Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            app.status = format!("Failed to copy to clipboard: {err}");
            return;
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    let _ = child.wait();
}

pub(super) fn quick_look_selected(app: &mut TuiApp) {
    let Some(result) = app.selected_result() else {
        app.status = "No selection to preview.".to_string();
        return;
    };
    let path = result.path.clone();
    match Command::new("qlmanage")
        .arg("-p")
        .arg(&path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => {
            app.status = format!("Quick Look: {}", path.display());
        }
        Err(err) => {
            app.status = format!("Failed to preview {}: {err}", path.display());
        }
    }
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
