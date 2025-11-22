mod app;
mod components;
mod config;
mod event;
mod model;
pub mod pipeline;
mod pipeline_worker;

fn main() {
    let picker = ratatui_image::picker::Picker::from_query_stdio().unwrap_or_else(|e| {
        eprintln!("Failed to query terminal ({}), using fallback", e);
        ratatui_image::picker::Picker::from_fontsize((8, 16))
    });

    let mut app = app::App::new_with_picker(picker);
    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
