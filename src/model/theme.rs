use std::fmt;

#[derive(Clone, Debug, Default)]
pub struct IndexTheme {
    pub name: String,
    pub comment: String,
    pub inherits: String,
    pub directories: Vec<String>,
}

impl fmt::Display for IndexTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[Icon Theme]")?;
        writeln!(f, "Name={}", self.name)?;
        writeln!(f, "Comment={}", self.comment)?;
        if !self.inherits.is_empty() {
            writeln!(f, "Inherits={}", self.inherits)?;
        }
        writeln!(f, "")?;
        writeln!(f, "# Directory list")?;
        writeln!(f, "Directories={}", self.directories.join(","))?;
        writeln!(f, "")?;

        for dir in &self.directories {
            writeln!(f, "[{}]", dir)?;
            writeln!(f, "Context=Cursors")?;
            writeln!(f, "Type=Fixed")?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct CursorTheme {
    pub name: String,
    pub comment: String,
    pub inherits: String,
}

impl fmt::Display for CursorTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[Icon Theme]")?;
        writeln!(f, "Name={}", self.name)?;
        writeln!(f, "Comment={}", self.comment)?;
        if !self.inherits.is_empty() {
            writeln!(f, "Inherits={}", self.inherits)?;
        }
        Ok(())
    }
}
