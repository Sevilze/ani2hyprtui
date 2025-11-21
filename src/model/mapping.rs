use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorMapping {
    #[serde(default = "default_x11_to_win")]
    pub x11_to_win: BTreeMap<String, String>,

    #[serde(default = "default_symlinks")]
    pub symlinks: BTreeMap<String, Vec<String>>,
}

impl Default for CursorMapping {
    fn default() -> Self {
        Self {
            x11_to_win: default_x11_to_win(),
            symlinks: default_symlinks(),
        }
    }
}

impl CursorMapping {
    pub fn get_win_name(&self, x11_name: &str) -> Option<&String> {
        self.x11_to_win.get(x11_name)
    }

    pub fn set_mapping(&mut self, x11_name: String, win_name: String) {
        self.x11_to_win.insert(x11_name, win_name);
    }

    pub fn get_symlinks(&self, x11_name: &str) -> Vec<String> {
        self.symlinks.get(x11_name).cloned().unwrap_or_default()
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let content = self
            .to_toml_string()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

fn default_x11_to_win() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    // Standard mappings
    map.insert("left_ptr".to_string(), "Normal".to_string());
    map.insert("link".to_string(), "Link Select".to_string()); // Prefer Link Select over Link
    map.insert("pointer".to_string(), "Person".to_string());
    map.insert("pencil".to_string(), "Handwriting".to_string());
    map.insert("text".to_string(), "Text".to_string());
    map.insert("not-allowed".to_string(), "Unavailable".to_string());
    map.insert("wait".to_string(), "Busy".to_string());
    map.insert("progress".to_string(), "Working in Background".to_string());
    map.insert("crosshair".to_string(), "Precision".to_string());
    map.insert("move".to_string(), "Move".to_string());
    map.insert("question_arrow".to_string(), "Alternate".to_string());
    map.insert("help".to_string(), "Help".to_string());
    map.insert("pin".to_string(), "Pin".to_string());
    map.insert("size_hor".to_string(), "Horizontal".to_string());
    map.insert("size_ver".to_string(), "Vertical".to_string());
    map.insert("size_bdiag".to_string(), "Diagonal1".to_string());
    map.insert("size_fdiag".to_string(), "Diagonal2".to_string());

    // Extended mappings for missing roots (Mapped to "Normal" fallback by default,
    // user can change if they have specific source files)
    map.insert("copy".to_string(), "Normal".to_string());
    map.insert("cell".to_string(), "Normal".to_string());
    map.insert("grabbing".to_string(), "Normal".to_string());
    map.insert("dotbox".to_string(), "Normal".to_string());
    map.insert("sb_down_arrow".to_string(), "Normal".to_string());
    map.insert("sb_left_arrow".to_string(), "Normal".to_string());
    map.insert("sb_right_arrow".to_string(), "Normal".to_string());
    map.insert("sb_up_arrow".to_string(), "Normal".to_string());
    map.insert("right_ptr".to_string(), "Normal".to_string());
    map.insert("right_side".to_string(), "Normal".to_string());
    map.insert("top_right_corner".to_string(), "Normal".to_string());
    map.insert("top_side".to_string(), "Normal".to_string());
    map.insert("top_left_corner".to_string(), "Normal".to_string());
    map.insert("bottom_right_corner".to_string(), "Normal".to_string());
    map.insert("bottom_side".to_string(), "Normal".to_string());
    map.insert("bottom_left_corner".to_string(), "Normal".to_string());
    map.insert("left_side".to_string(), "Normal".to_string());
    map.insert("X_cursor".to_string(), "Normal".to_string());

    map
}

fn default_symlinks() -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();

    map.insert(
        "left_ptr".to_string(),
        vec![
            "arrow".to_string(),
            "default".to_string(),
            "top_left_arrow".to_string(),
            "wayland-cursor".to_string(),
        ],
    );

    map.insert(
        "pointer".to_string(),
        vec![
            "hand1".to_string(),
            "hand2".to_string(),
            "pointing_hand".to_string(),
            "openhand".to_string(),
            "grab".to_string(),
            "9d800788f1b08800ae810202380a0822".to_string(), // hand2
            "e29285e634086352946a0e7090d73106".to_string(), // hand2
        ],
    );

    map.insert(
        "move".to_string(),
        vec![
            "fleur".to_string(),
            "all-scroll".to_string(),
            "size_all".to_string(),
            "4498f0e0c1937ffe01fd06f973665830".to_string(),
            "9081237383d90e509aa00f00170e968f".to_string(),
        ],
    );

    map.insert("wait".to_string(), vec!["watch".to_string()]);

    map.insert(
        "progress".to_string(),
        vec![
            "left_ptr_watch".to_string(),
            "00000000000000020006000e7e9ffc3f".to_string(),
            "08e8e1c95fe2fc01f976f1e063a24ccd".to_string(),
            "3ecb610c1bf2410f44200f48c40d3599".to_string(),
        ],
    );

    map.insert(
        "crosshair".to_string(),
        vec![
            "cross".to_string(),
            "cross_reverse".to_string(),
            "diamond_cross".to_string(),
        ],
    );

    map.insert(
        "text".to_string(),
        vec!["xterm".to_string(), "ibeam".to_string()],
    );

    map.insert("pencil".to_string(), vec!["draft".to_string()]);

    map.insert(
        "question_arrow".to_string(),
        vec![
            "help".to_string(),
            "whats_this".to_string(),
            "left_ptr_help".to_string(),
            "5c6cd98b3f3ebcb1f9c7f1c204630408".to_string(),
            "d9ce0ab605698f320427677b458ad60b".to_string(),
        ],
    );

    map.insert(
        "not-allowed".to_string(),
        vec![
            "crossed_circle".to_string(),
            "forbidden".to_string(),
            "no_drop".to_string(),
            "dnd_no_drop".to_string(),
            "03b6e0fcb3499374a867c041f52298f0".to_string(), // crossed_circle
            "no-drop".to_string(),
        ],
    );

    map.insert(
        "size_hor".to_string(),
        vec![
            "sb_h_double_arrow".to_string(),
            "h_double_arrow".to_string(),
            "ew-resize".to_string(),
            "col-resize".to_string(),
            "split_h".to_string(),
            "size-hor".to_string(),
            "028006030e0e7ebffc7f7070c0600140".to_string(), // sb_h_double_arrow
            "14fef782d02440884392942c1120523".to_string(),  // sb_h_double_arrow
        ],
    );

    map.insert(
        "size_ver".to_string(),
        vec![
            "sb_v_double_arrow".to_string(),
            "v_double_arrow".to_string(),
            "ns-resize".to_string(),
            "row-resize".to_string(),
            "split_v".to_string(),
            "size-ver".to_string(),
            "00008160000006810000408080010102".to_string(), // sb_v_double_arrow
            "2870a09082c103050810ffdffffe0204".to_string(), // sb_v_double_arrow
            "double_arrow".to_string(),
        ],
    );

    map.insert(
        "size_fdiag".to_string(),
        vec![
            "fd_double_arrow".to_string(),
            "nesw-resize".to_string(),
            "c7088f0f3e6c8088236ef8e1e3e70000".to_string(),
        ],
    );

    map.insert(
        "size_bdiag".to_string(),
        vec![
            "bd_double_arrow".to_string(),
            "nwse-resize".to_string(),
            "fcf1c3c7cd4491d801f1e1c78f100000".to_string(),
        ],
    );

    // Additional links from add_missing_links.sh
    map.insert(
        "link".to_string(),
        vec![
            "3085a0e285430894940527032f8b26df".to_string(),
            "640fb0e74195791501fd1ed57b41487f".to_string(),
            "a2a266d0498c3104214a47bd64ab0fc8".to_string(),
            "dnd-link".to_string(),
            "alias".to_string(),
        ],
    );

    map.insert(
        "copy".to_string(),
        vec![
            "1081e37283d90000800003c07f3ef6bf".to_string(),
            "6407b0e94181790501fd1e167b474872".to_string(),
            "b66166c04f8c3109214a4fbd64a50fc8".to_string(),
        ],
    );

    map.insert("cell".to_string(), vec!["plus".to_string()]);
    map.insert("color-picker".to_string(), vec!["tcross".to_string()]);
    map.insert(
        "grabbing".to_string(),
        vec![
            "closedhand".to_string(),
            "dnd-move".to_string(),
            "dnd-none".to_string(),
            "fcf21c00b30f7e3f83fe0dfd12e71cff".to_string(),
        ],
    );

    // Fixed dotbox direction: dotbox is target, dot_box_mask is link
    map.insert(
        "dotbox".to_string(),
        vec![
            "dot_box_mask".to_string(),
            "draped_box".to_string(),
            "icon".to_string(),
            "target".to_string(),
        ],
    );

    map.insert("sb_down_arrow".to_string(), vec!["down-arrow".to_string()]);
    map.insert("sb_left_arrow".to_string(), vec!["left-arrow".to_string()]);
    map.insert(
        "sb_right_arrow".to_string(),
        vec!["right-arrow".to_string()],
    );
    map.insert("sb_up_arrow".to_string(), vec!["up-arrow".to_string()]);

    map.insert(
        "right_ptr".to_string(),
        vec!["draft_large".to_string(), "draft_small".to_string()],
    );

    map.insert("right_side".to_string(), vec!["e-resize".to_string()]);
    map.insert(
        "top_right_corner".to_string(),
        vec!["ne-resize".to_string()],
    );
    map.insert("top_side".to_string(), vec!["n-resize".to_string()]);
    map.insert("top_left_corner".to_string(), vec!["nw-resize".to_string()]);
    map.insert(
        "bottom_right_corner".to_string(),
        vec!["se-resize".to_string()],
    );
    map.insert("bottom_side".to_string(), vec!["s-resize".to_string()]);
    map.insert(
        "bottom_left_corner".to_string(),
        vec!["sw-resize".to_string()],
    );
    map.insert("left_side".to_string(), vec!["w-resize".to_string()]);

    map.insert(
        "X_cursor".to_string(),
        vec!["pirate".to_string(), "x-cursor".to_string()],
    );

    map
}
