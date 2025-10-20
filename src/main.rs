mod app;
mod config;

fn main() {
    let mut app = app::App::new();
    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
