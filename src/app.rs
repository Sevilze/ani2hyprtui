use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{io, time::Duration};

use crate::config::Config;

pub struct App {
    pub config: Config,
}

impl App {
    pub fn new() -> Self {
        Self { config: Config::default() }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        let tick_rate = Duration::from_millis(200);
        let mut res: Result<()> = Ok(());

        'outer: loop {
            terminal.draw(|f| {
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ])
                    .split(area);

                let header = Paragraph::new("ani2hyprtui")
                    .style(Style::default().fg(Color::Cyan))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).title("Header"));
                f.render_widget(header, chunks[0]);

                let body = Paragraph::new("Initial UI ready. Use 'q' to quit.")
                    .block(Block::default().borders(Borders::ALL).title("Main"));
                f.render_widget(body, chunks[1]);

                let status = Paragraph::new("Press q to quit  |  Ctrl+C also exits")
                    .style(Style::default().fg(Color::Gray))
                    .block(Block::default().borders(Borders::ALL).title("Status"));
                f.render_widget(status, chunks[2]);
            })?;

            if event::poll(tick_rate)? {
                match event::read()? {
                    Event::Key(KeyEvent { code, modifiers, .. }) => {
                        match (code, modifiers) {
                            (KeyCode::Char('q'), _) => break 'outer,
                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => break 'outer,
                            _ => {}
                        }
                    }
                    Event::Resize(_, _) => {
                    }
                    _ => {}
                }
            } else {
                // Tick timeout; used later for animation
            }
        }

        // Restore terminal
        if let Err(e) = restore_terminal(&mut terminal) {
            res = Err(e.into());
        }
        res
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    terminal.show_cursor().ok();
    disable_raw_mode().ok();
    // LeaveAlternateScreen must be executed on the same stdout the backend uses
    let mut out = io::stdout();
    execute!(out, LeaveAlternateScreen)?;
    Ok(())
}

