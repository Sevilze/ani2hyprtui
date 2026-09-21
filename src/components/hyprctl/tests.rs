use super::*;
use crate::event::AppMsg;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_scan_cursor_themes_with_extra_dir() {
    let temp = tempdir().unwrap();
    let theme_dir = temp.path().join("LiveTestTheme");
    fs::create_dir_all(&theme_dir).unwrap();
    fs::write(theme_dir.join("manifest.hl"), "name = LiveTestTheme\n").unwrap();

    let items = discovery::scan_cursor_themes_with_extra(&[temp.path().to_path_buf()]);
    let found = items.iter().find(|t| t.name == "LiveTestTheme");
    assert!(found.is_some());
    let item = found.unwrap();
    assert!(item.is_hyprcursor);
    assert_eq!(item.path, theme_dir);
}

#[test]
fn test_refresh_themes_live_discovery_and_preservation() {
    let temp = tempdir().unwrap();
    let mut state = HyprctlState {
        extra_dirs: vec![temp.path().to_path_buf()],
        ..Default::default()
    };

    let theme_a = temp.path().join("LiveThemeA");
    fs::create_dir_all(&theme_a).unwrap();
    fs::write(theme_a.join("manifest.hl"), "name = LiveThemeA\n").unwrap();

    state.refresh_themes();
    let idx_a = state.themes.iter().position(|t| t.name == "LiveThemeA");
    assert!(idx_a.is_some());

    state.select_theme_idx(idx_a.unwrap());
    assert_eq!(state.current_theme().unwrap().name, "LiveThemeA");

    // Add a second theme and refresh
    let theme_b = temp.path().join("LiveThemeB");
    fs::create_dir_all(&theme_b).unwrap();
    fs::write(theme_b.join("manifest.hl"), "name = LiveThemeB\n").unwrap();

    state.refresh_themes();
    assert_eq!(state.current_theme().unwrap().name, "LiveThemeA");
}

#[test]
fn test_is_applying_blocks_concurrent_execution() {
    let mut state = HyprctlState {
        is_applying: true,
        ..Default::default()
    };
    assert!(state.apply_cursor().is_none());
}

#[test]
fn test_refresh_themes_detects_deleted_directory() {
    let temp = tempdir().unwrap();
    let mut state = HyprctlState {
        extra_dirs: vec![temp.path().to_path_buf()],
        ..Default::default()
    };

    let theme_dir = temp.path().join("ThemeToDelete");
    fs::create_dir_all(&theme_dir).unwrap();
    fs::write(theme_dir.join("manifest.hl"), "name = ThemeToDelete\n").unwrap();

    state.refresh_themes();
    assert!(state.themes.iter().any(|t| t.name == "ThemeToDelete"));

    let idx = state
        .themes
        .iter()
        .position(|t| t.name == "ThemeToDelete")
        .unwrap();
    state.select_theme_idx(idx);

    fs::remove_dir_all(&theme_dir).unwrap();

    state.refresh_themes();
    assert!(!state.themes.iter().any(|t| t.name == "ThemeToDelete"));
    let (msg, is_err) = state.status_message.unwrap();
    assert!(is_err);
    assert!(msg.contains("deleted from disk"));
}

#[test]
fn test_apply_cursor_rejects_deleted_directory() {
    let temp = tempdir().unwrap();
    let mut state = HyprctlState {
        extra_dirs: vec![temp.path().to_path_buf()],
        ..Default::default()
    };

    let theme_dir = temp.path().join("DeletedBeforeApply");
    fs::create_dir_all(&theme_dir).unwrap();
    fs::write(theme_dir.join("manifest.hl"), "name = DeletedBeforeApply\n").unwrap();

    state.refresh_themes();
    let idx = state
        .themes
        .iter()
        .position(|t| t.name == "DeletedBeforeApply")
        .unwrap();
    state.select_theme_idx(idx);

    fs::remove_dir_all(&theme_dir).unwrap();

    let res = state.apply_cursor();
    assert!(res.is_some());
    if let Some(AppMsg::LogMessage(log)) = res {
        assert!(log.contains("deleted"));
    } else {
        panic!("Expected LogMessage with deletion error");
    }
    let (msg, is_err) = state.status_message.unwrap();
    assert!(is_err);
    assert!(msg.contains("deleted from disk"));
}

#[test]
fn test_ensure_theme_accessible_rejects_non_existent_directory() {
    let fake_path = PathBuf::from("/tmp/non_existent_cursor_dir_12345");
    let res = apply::ensure_theme_accessible("non_existent", &fake_path);
    assert!(res.is_err());
}
