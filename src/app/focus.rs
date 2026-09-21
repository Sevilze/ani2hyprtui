#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePipeline {
    Full,
    XCursor,
    Png,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileBrowser,
    Runner,
    Overrides,
    Editor,
    Logs,
    Mapping,
    Settings,
    Hyprctl,
}

impl Focus {
    pub fn next(&self) -> Self {
        match self {
            Focus::FileBrowser => Focus::Runner,
            Focus::Runner => Focus::Overrides,
            Focus::Overrides => Focus::Editor,
            Focus::Editor => Focus::Logs,
            Focus::Logs => Focus::Mapping,
            Focus::Mapping => Focus::Settings,
            Focus::Settings => Focus::Hyprctl,
            Focus::Hyprctl => Focus::FileBrowser,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Focus::FileBrowser => Focus::Hyprctl,
            Focus::Hyprctl => Focus::Settings,
            Focus::Settings => Focus::Mapping,
            Focus::Mapping => Focus::Logs,
            Focus::Logs => Focus::Editor,
            Focus::Editor => Focus::Overrides,
            Focus::Overrides => Focus::Runner,
            Focus::Runner => Focus::FileBrowser,
        }
    }

    pub fn left(&self) -> Option<Self> {
        match self {
            Focus::Editor => Some(Focus::FileBrowser),
            Focus::Logs => Some(Focus::Overrides),
            Focus::Mapping => Some(Focus::Editor),
            Focus::Settings => Some(Focus::Logs),
            Focus::Hyprctl => Some(Focus::Logs),
            _ => None,
        }
    }

    pub fn right(&self) -> Option<Self> {
        match self {
            Focus::FileBrowser => Some(Focus::Editor),
            Focus::Runner => Some(Focus::Editor),
            Focus::Overrides => Some(Focus::Logs),
            Focus::Editor => Some(Focus::Mapping),
            Focus::Logs => Some(Focus::Settings),
            _ => None,
        }
    }

    pub fn up(&self) -> Option<Self> {
        match self {
            Focus::Runner => Some(Focus::FileBrowser),
            Focus::Overrides => Some(Focus::Runner),
            Focus::Logs => Some(Focus::Editor),
            Focus::Settings => Some(Focus::Mapping),
            Focus::Hyprctl => Some(Focus::Settings),
            _ => None,
        }
    }

    pub fn down(&self) -> Option<Self> {
        match self {
            Focus::FileBrowser => Some(Focus::Runner),
            Focus::Runner => Some(Focus::Overrides),
            Focus::Editor => Some(Focus::Logs),
            Focus::Mapping => Some(Focus::Settings),
            Focus::Settings => Some(Focus::Hyprctl),
            _ => None,
        }
    }
}
