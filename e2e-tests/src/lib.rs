//! End-to-end tests that drive the Cardinal desktop app via xa11y.
//!
//! Prerequisites:
//! - Cardinal must be running (`npm run tauri dev -- --release --features dev`)
//! - macOS: Terminal must have Accessibility permission
//!
//! Run: cargo test -p e2e-tests

use std::time::Duration;
use xa11y::{App, Error, Result, provider};

const APP_NAME: &str = "Cardinal";
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Selector for the search input field (there is exactly one text_field).
const SEARCH_INPUT: &str = "text_field";

/// Selector for the status bar text showing result count (e.g. "5 results • 3ms").
const RESULTS_TEXT: &str = r#"static_text[value*="result"]"#;

fn find_app() -> Result<App> {
    let p = provider()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(app) = App::from_name(p.clone(), APP_NAME) {
            return Ok(app);
        }
        if std::time::Instant::now() > deadline {
            return Err(Error::AppNotFound {
                target: APP_NAME.to_string(),
            });
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Poll until the status bar shows a result count, indicating search completed.
fn wait_for_results(app: &App, timeout: Duration, query: &str) {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::time::Instant::now() > deadline {
            panic!("Search for '{query}' did not complete within {timeout:?}");
        }
        let status = app.locator(RESULTS_TEXT);
        if !status.elements().unwrap_or_default().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[test]
fn app_launches_and_shows_search_input() -> Result<()> {
    let app = find_app()?;
    let elements = app.locator(SEARCH_INPUT).elements()?;
    assert!(
        !elements.is_empty(),
        "Search input should be visible when the app launches"
    );
    Ok(())
}

#[test]
fn search_star_js_returns_results() -> Result<()> {
    let app = find_app()?;
    app.locator(SEARCH_INPUT).set_value("*.js")?;
    wait_for_results(&app, SEARCH_TIMEOUT, "*.js");
    Ok(())
}

#[test]
fn search_path_filter_does_not_hang() -> Result<()> {
    let app = find_app()?;
    app.locator(SEARCH_INPUT).set_value("path:repos")?;
    wait_for_results(&app, SEARCH_TIMEOUT, "path:repos");
    Ok(())
}

#[test]
fn changing_search_dismisses_spinner() -> Result<()> {
    let app = find_app()?;
    let search = app.locator(SEARCH_INPUT);
    search.set_value("path:repos")?;
    std::thread::sleep(Duration::from_millis(500));
    search.set_value("*.js")?;
    wait_for_results(&app, SEARCH_TIMEOUT, "*.js (after path:repos)");
    Ok(())
}

#[test]
fn main_js_path_ayla_path_repos_query() -> Result<()> {
    let app = find_app()?;
    app.locator(SEARCH_INPUT)
        .set_value("main.js path:Ayla path:repos")?;
    wait_for_results(&app, SEARCH_TIMEOUT, "main.js path:Ayla path:repos");
    Ok(())
}
