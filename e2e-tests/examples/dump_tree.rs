use xa11y::{App, provider};

fn main() {
    let p = provider().unwrap();
    let app = App::from_name(p, "Cardinal").unwrap();

    // Try various roles to find the search input
    for role in [
        "text_field",
        "text_area",
        "button",
        "static_text",
        "group",
        "window",
        "list",
        "list_item",
    ] {
        let elements = app.locator(role).elements().unwrap_or_default();
        if !elements.is_empty() {
            println!("\n=== Role: {role} ({} elements) ===", elements.len());
            for e in elements.iter().take(20) {
                println!(
                    "  role={:?} name={:?} desc={:?} value={:?}",
                    e.role, e.name, e.description, e.value
                );
            }
        }
    }
}
