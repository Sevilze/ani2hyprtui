mod app;
mod components;
mod config;
mod event;
mod model;
pub mod pipeline;
mod pipeline_worker;
mod widgets;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--version".to_string()) {
        println!("ani2hyprtui {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let picker = ratatui_image::picker::Picker::from_query_stdio().unwrap_or_else(|e| {
        eprintln!("Failed to query terminal ({}), using fallback", e);
        ratatui_image::picker::Picker::halfblocks()
    });

    let mut app = app::App::new_with_picker(picker);
    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
