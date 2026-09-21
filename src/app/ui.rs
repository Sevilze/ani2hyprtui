use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    widgets::Paragraph,
};

use super::App;
use super::focus::Focus;
use crate::components::Component;
use crate::widgets::theme::get_theme;

pub fn draw_ui(app: &mut App, f: &mut Frame) {
    let area = f.area();
    let theme = get_theme();

    f.buffer_mut()
        .set_style(area, Style::default().bg(theme.surface));

    // Main layout: vertical split into content and status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    if app.cursor_editor.maximized {
        app.cursor_editor
            .render(main_chunks[0], f.buffer_mut(), true);
    } else {
        // Always show all three columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Left: File Browser, Runner, Overrides
                Constraint::Percentage(50), // Middle: Cursor Editor, Logs
                Constraint::Percentage(25), // Right: Mapping Editor, Settings, Hyprctl
            ])
            .split(main_chunks[0]);

        // Left Column
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // File Browser
                Constraint::Percentage(20), // Runner
                Constraint::Percentage(40), // Overrides
            ])
            .split(columns[0]);

        // Middle Column
        let middle_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Cursor Editor
                Constraint::Percentage(30), // Logs
            ])
            .split(columns[1]);

        // Right Column
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Mapping Editor
                Constraint::Percentage(25), // Settings
                Constraint::Percentage(25), // Hyprctl
            ])
            .split(columns[2]);

        // Render components
        app.file_browser.render(
            left_chunks[0],
            f.buffer_mut(),
            app.focus == Focus::FileBrowser,
        );
        app.runner
            .render(left_chunks[1], f.buffer_mut(), app.focus == Focus::Runner);
        app.theme_overrides.render(
            left_chunks[2],
            f.buffer_mut(),
            app.focus == Focus::Overrides,
        );

        app.cursor_editor
            .render(middle_chunks[0], f.buffer_mut(), app.focus == Focus::Editor);
        app.logs
            .render(middle_chunks[1], f.buffer_mut(), app.focus == Focus::Logs);

        app.mapping_editor
            .render(right_chunks[0], f.buffer_mut(), app.focus == Focus::Mapping);
        app.settings.render(
            right_chunks[1],
            f.buffer_mut(),
            app.focus == Focus::Settings,
        );
        app.hyprctl
            .render(right_chunks[2], f.buffer_mut(), app.focus == Focus::Hyprctl);
    }

    // Status bar
    let focus_str = format!("{:?}", app.focus);
    let status_text = format!(
        "q: Quit | Ctrl+hjkl: Navigate | Focus: {} | {}",
        focus_str,
        match app.focus {
            Focus::FileBrowser => "i/o: Set In/Out | Enter: Select | l: Load",
            Focus::Runner => "c: Full Convert | x: XCur | p: PNG",
            Focus::Overrides => "Tab: Switch Field | Type to edit",
            Focus::Editor => "Space: Play | ,/.: Frame | [/]: Size | Arrows: Hotspot | S: Save",
            Focus::Logs => "Logs View",
            Focus::Settings => "↑↓/jk: Select | Enter: Apply & Save | ←→/hl: Quick Switch",
            Focus::Hyprctl => "↑↓/jk: Theme | ←→/hl: Size | Enter: Apply (hyprctl)",
            Focus::Mapping => "Enter: Edit | s: Save",
        }
    );

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(theme.text_secondary))
        .alignment(Alignment::Center);
    f.render_widget(status, main_chunks[1]);
}
