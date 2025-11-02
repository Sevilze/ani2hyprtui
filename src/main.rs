mod app;
mod components;
mod config;
mod event;
mod model;
pub mod pipeline;
mod pipeline_worker;

fn main() {
    let mut app = app::App::new();
    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
