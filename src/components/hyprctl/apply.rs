use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::AppConfig;
use crate::event::AppMsg;

use super::HyprctlState;

pub fn ensure_theme_accessible(theme_name: &str, theme_path: &Path) -> std::io::Result<()> {
    if !theme_path.exists() || !theme_path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Theme directory '{}' does not exist", theme_path.display()),
        ));
    }

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Ok(()),
    };

    let user_icons = home.join(".local/share/icons");
    let legacy_icons = home.join(".icons");

    if theme_path.starts_with(&user_icons)
        || theme_path.starts_with(&legacy_icons)
        || theme_path.starts_with("/usr/share/icons")
        || theme_path.starts_with("/usr/local/share/icons")
    {
        return Ok(());
    }

    if let Ok(extra) = std::env::var("XDG_DATA_DIRS") {
        for part in extra.split(':') {
            let p = Path::new(part.trim());
            if !part.trim().is_empty() && theme_path.starts_with(p.join("icons")) {
                return Ok(());
            }
        }
    }

    if let Ok(extra) = std::env::var("XCURSOR_PATH") {
        for part in extra.split(':') {
            let p = Path::new(part.trim());
            if !part.trim().is_empty() && theme_path.starts_with(p) {
                return Ok(());
            }
        }
    }

    fs::create_dir_all(&user_icons)?;
    let link_target = theme_path
        .canonicalize()
        .unwrap_or_else(|_| theme_path.to_path_buf());
    let symlink_path = user_icons.join(theme_name);

    if symlink_path.is_symlink() {
        if let Ok(existing_target) = fs::read_link(&symlink_path)
            && existing_target == link_target
            && link_target.exists()
        {
            return Ok(());
        }
        let _ = fs::remove_file(&symlink_path);
    } else if symlink_path.exists() {
        return Ok(());
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&link_target, &symlink_path)?;

    Ok(())
}

pub fn run_hyprctl_setcursor(theme_name: &str, size: u32) -> (bool, String) {
    let result = Command::new("hyprctl")
        .args(["setcursor", theme_name, &size.to_string()])
        .output();

    match result {
        Ok(output) if output.status.success() => (true, format!("Applied: {}px", size)),
        Ok(output) => {
            let err_text = String::from_utf8_lossy(&output.stderr);
            let err_msg = if err_text.trim().is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                err_text.trim().to_string()
            };
            (false, format!("Failed: {}", err_msg))
        }
        Err(e) => (false, format!("Cannot run hyprctl: {}", e)),
    }
}

pub fn apply_cursor(state: &mut HyprctlState) -> Option<AppMsg> {
    if state.is_applying {
        return None;
    }

    let pre_selected_name = state.current_theme().map(|t| t.name.clone());

    state.refresh_themes();

    if let Some(ref intended_name) = pre_selected_name
        && !state.themes.iter().any(|t| &t.name == intended_name)
    {
        let err_msg = format!("Theme '{}' was deleted from disk", intended_name);
        state.status_message = Some((err_msg.clone(), true));
        return Some(AppMsg::LogMessage(format!(
            "Cannot apply cursor: {}",
            err_msg
        )));
    }

    let theme = match state.current_theme() {
        Some(t) => t,
        None => {
            state.status_message = Some(("No theme selected".to_string(), true));
            return Some(AppMsg::LogMessage(
                "Cannot apply cursor: no theme selected".to_string(),
            ));
        }
    };

    let theme_name = theme.name.clone();
    let theme_path = theme.path.clone();

    if !theme_path.exists() || !theme_path.is_dir() {
        state.refresh_themes();
        let err_msg = format!("Theme '{}' was deleted from disk", theme_name);
        state.status_message = Some((err_msg.clone(), true));
        return Some(AppMsg::LogMessage(format!(
            "Cannot apply cursor: {} ({})",
            err_msg,
            theme_path.display()
        )));
    }

    let has_hypr =
        theme_path.join("manifest.hl").exists() || theme_path.join("hyprcursors").is_dir();
    let has_xcur = theme_path.join("cursors").is_dir();
    if !has_hypr && !has_xcur {
        state.refresh_themes();
        let err_msg = format!("Theme '{}' has no cursor files", theme_name);
        state.status_message = Some((err_msg.clone(), true));
        return Some(AppMsg::LogMessage(format!(
            "Cannot apply cursor: {}",
            err_msg
        )));
    }

    let size = theme
        .available_sizes
        .get(state.selected_size_idx)
        .copied()
        .unwrap_or(24);

    state.is_applying = true;
    state.status_message = Some((format!("Applying {} ({}px)...", theme_name, size), false));

    let log_theme_name = theme_name.clone();
    if let Some(tx) = state.tx.clone() {
        std::thread::spawn(move || {
            if let Err(e) = ensure_theme_accessible(&theme_name, &theme_path) {
                let err_msg = format!("Theme setup failed: {}", e);
                let _ = tx.send(AppMsg::CursorThemeApplied {
                    theme_name,
                    size,
                    success: false,
                    message: err_msg,
                });
                return;
            }

            let (success, message) = run_hyprctl_setcursor(&theme_name, size);

            let _ = tx.send(AppMsg::CursorThemeApplied {
                theme_name,
                size,
                success,
                message,
            });
        });

        Some(AppMsg::LogMessage(format!(
            "Applying cursor theme {} ({}px)...",
            log_theme_name, size
        )))
    } else {
        let _ = ensure_theme_accessible(&theme_name, &theme_path);
        let (success, message) = run_hyprctl_setcursor(&theme_name, size);

        state.is_applying = false;
        if success {
            state.active_theme = theme_name.clone();
            state.active_size = size;
            state.status_message = Some((message.clone(), false));
            AppConfig::save_cursor(&theme_name, size);
            Some(AppMsg::LogMessage(format!(
                "hyprctl setcursor {} {} ({})",
                theme_name, size, message
            )))
        } else {
            state.status_message = Some((message.clone(), true));
            Some(AppMsg::LogMessage(format!(
                "hyprctl setcursor failed: {}",
                message
            )))
        }
    }
}
