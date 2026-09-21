use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;
use crate::pipeline::hyprcursor::detect_theme_sizes;

use super::types::CursorThemeItem;

pub fn detect_active_cursor() -> (String, u32) {
    let app_cfg = AppConfig::load();
    if let (Some(th), Some(sz)) = (app_cfg.cursor_theme, app_cfg.cursor_size)
        && !th.trim().is_empty()
    {
        return (th, sz);
    }

    let env_theme = std::env::var("HYPRCURSOR_THEME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("XCURSOR_THEME")
                .ok()
                .filter(|s| !s.trim().is_empty())
        });
    let env_size = std::env::var("HYPRCURSOR_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| {
            std::env::var("XCURSOR_SIZE")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
        });

    if let Some(t) = env_theme {
        return (t, env_size.unwrap_or(24));
    }

    if let Some(home) = dirs::home_dir() {
        let hypr_files = [
            home.join(".config/hypr/custom/env.lua"),
            home.join(".config/hypr/env.lua"),
            home.join(".config/hypr/hyprland.conf"),
        ];
        for path in &hypr_files {
            if let Ok(content) = fs::read_to_string(path) {
                let mut found_theme = None;
                let mut found_size = None;
                for line in content.lines() {
                    let line = line.trim();
                    if (line.contains("HYPRCURSOR_THEME") || line.contains("XCURSOR_THEME"))
                        && found_theme.is_none()
                        && let Some(val) = extract_config_value(line)
                    {
                        found_theme = Some(val);
                    }
                    if (line.contains("HYPRCURSOR_SIZE") || line.contains("XCURSOR_SIZE"))
                        && found_size.is_none()
                        && let Some(val) = extract_config_value(line)
                        && let Ok(sz) = val.parse::<u32>()
                    {
                        found_size = Some(sz);
                    }
                }
                if let Some(th) = found_theme {
                    return (th, found_size.unwrap_or(24));
                }
            }
        }
    }

    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
        .output()
    {
        let th = String::from_utf8_lossy(&output.stdout)
            .trim()
            .trim_matches('\'')
            .trim()
            .to_string();
        if !th.is_empty() {
            let sz = Command::new("gsettings")
                .args(["get", "org.gnome.desktop.interface", "cursor-size"])
                .output()
                .ok()
                .and_then(|out| {
                    String::from_utf8_lossy(&out.stdout)
                        .trim()
                        .parse::<u32>()
                        .ok()
                })
                .unwrap_or(24);
            return (th, sz);
        }
    }

    (String::new(), 24)
}

pub fn scan_cursor_themes() -> Vec<CursorThemeItem> {
    scan_cursor_themes_with_extra(&[])
}

pub fn scan_cursor_themes_with_extra(extra_dirs: &[PathBuf]) -> Vec<CursorThemeItem> {
    let mut dirs = Vec::new();

    // 1. Extra directories (runner output, input, file_browser current dir)
    for ed in extra_dirs {
        let p = ed.canonicalize().unwrap_or_else(|_| ed.clone());
        if !dirs.contains(&p) {
            dirs.push(p);
        }
    }

    // 2. User local directories
    let user_icons = dirs::home_dir().map(|h| h.join(".local/share/icons"));
    let legacy_icons = dirs::home_dir().map(|h| h.join(".icons"));

    if let Some(ref u) = user_icons
        && !dirs.contains(u)
    {
        dirs.push(u.clone());
    }
    if let Some(ref l) = legacy_icons
        && !dirs.contains(l)
    {
        dirs.push(l.clone());
    }

    // 3. System directories
    dirs.push(PathBuf::from("/usr/local/share/icons"));
    dirs.push(PathBuf::from("/usr/share/icons"));

    // 4. XDG_DATA_DIRS
    if let Ok(extra) = std::env::var("XDG_DATA_DIRS") {
        for part in extra.split(':') {
            if !part.trim().is_empty() {
                let p = Path::new(part.trim()).join("icons");
                if !dirs.contains(&p) {
                    dirs.push(p);
                }
            }
        }
    }

    // 5. XCURSOR_PATH
    if let Ok(extra) = std::env::var("XCURSOR_PATH") {
        for part in extra.split(':') {
            if !part.trim().is_empty() {
                let p = PathBuf::from(part.trim());
                if !dirs.contains(&p) {
                    dirs.push(p);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for base in dirs {
        if !base.is_dir() {
            continue;
        }

        // Check if base itself is a cursor theme
        let is_base_hypr = base.join("manifest.hl").exists() || base.join("hyprcursors").is_dir();
        let is_base_xcur = base.join("cursors").is_dir();
        if (is_base_hypr || is_base_xcur)
            && let Some(name) = base.file_name().and_then(|n| n.to_str())
            && !name.starts_with('.')
            && !seen.contains(name)
        {
            seen.insert(name.to_string());
            let sizes = detect_theme_sizes(&base);
            items.push(CursorThemeItem {
                name: name.to_string(),
                path: base.clone(),
                is_hyprcursor: is_base_hypr,
                available_sizes: sizes,
            });
        }

        // Scan subdirectories
        let Ok(entries) = fs::read_dir(&base) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // If entry is a symlink whose target does not exist, clean it up!
            if path.is_symlink() && !path.exists() {
                if let Some(ref u) = user_icons
                    && base == *u
                {
                    let _ = fs::remove_file(&path);
                }
                if let Some(ref l) = legacy_icons
                    && base == *l
                {
                    let _ = fs::remove_file(&path);
                }
                continue;
            }

            if !path.is_dir() {
                continue;
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') || seen.contains(name) {
                continue;
            }

            let has_hypr = path.join("manifest.hl").exists() || path.join("hyprcursors").is_dir();
            let has_xcur = path.join("cursors").is_dir();

            if !has_hypr && !has_xcur {
                continue;
            }

            seen.insert(name.to_string());
            let sizes = detect_theme_sizes(&path);

            items.push(CursorThemeItem {
                name: name.to_string(),
                path: path.clone(),
                is_hyprcursor: has_hypr,
                available_sizes: sizes,
            });
        }
    }

    items.sort_by_key(|a| a.name.to_lowercase());
    items
}

pub fn extract_config_value(line: &str) -> Option<String> {
    let quotes: Vec<_> = line.match_indices('"').collect();
    if quotes.len() >= 4 {
        return Some(line[quotes[2].0 + 1..quotes[3].0].to_string());
    }
    let squotes: Vec<_> = line.match_indices('\'').collect();
    if squotes.len() >= 4 {
        return Some(line[squotes[2].0 + 1..squotes[3].0].to_string());
    }
    if let Some((_, val)) = line.split_once(',') {
        let v = val.trim().trim_matches('"').trim_matches('\'').trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}
