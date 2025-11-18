// XCursor theme generator - takes X11 cursor binaries from win2xcur converter
// and organizes them into a proper theme structure with mapping and symlinks

use anyhow::Result;
use crate::model::mapping::CursorMapping;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

pub struct XCursorThemeBuilder {
    output_dir: PathBuf,
    theme_name: String,
    mapping: CursorMapping,
}

impl XCursorThemeBuilder {
    pub fn new<P: Into<PathBuf>>(
        output_dir: P,
        theme_name: String,
        mapping: CursorMapping,
    ) -> Self {
        Self {
            output_dir: output_dir.into(),
            theme_name,
            mapping,
        }
    }
    
    /// Build theme from existing X11 cursor binaries
    /// xcur_source_dir should contain cursor files with Windows names
    pub fn build_from_xcur_files(&self, xcur_source_dir: &Path) -> Result<usize> {
        let cursors_dir = self.output_dir.join("cursors");
        fs::create_dir_all(&cursors_dir)?;
        
        let mut count = 0;
        
        // Copy and rename cursor files according to mapping
        for (win_name, x11_name) in &self.mapping.win_to_x11 {
            let source_file = xcur_source_dir.join(win_name);
            if !source_file.exists() {
                continue;
            }
            
            let dest_file = cursors_dir.join(x11_name);
            fs::copy(&source_file, &dest_file)?;
            count += 1;
        }
        
        self.create_symlinks(&cursors_dir)?;
        self.create_theme_files()?;
        self.install_to_user_icons()?;
        
        Ok(count)
    }
    
    fn create_symlinks(&self, cursors_dir: &Path) -> Result<()> {
        for (x11_name, symlink_names) in &self.mapping.symlinks {
            let target = x11_name; // Relative symlink
            let target_file = cursors_dir.join(x11_name);
            
            if !target_file.exists() {
                continue;
            }
            
            for symlink_name in symlink_names {
                let symlink_path = cursors_dir.join(symlink_name);
                
                if symlink_path.exists() {
                    continue;
                }
                
                unix_fs::symlink(target, &symlink_path)?;
            }
        }
        
        Ok(())
    }
    
    fn create_theme_files(&self) -> Result<()> {
        let index_theme_content = format!(
            "[Icon Theme]\n\
             Name={}\n\
             Comment={} cursor theme\n\
             Inherits=hicolor\n\
             \n\
             Directories=cursors\n\
             \n\
             [cursors]\n\
             Context=Cursors\n\
             Type=Fixed\n",
            self.theme_name, self.theme_name
        );
        
        fs::write(self.output_dir.join("index.theme"), index_theme_content)?;
        
        let cursor_theme_content = format!(
            "[Icon Theme]\n\
             Name={}\n\
             Comment={} cursor theme\n\
             Inherits=hicolor\n",
            self.theme_name, self.theme_name
        );
        
        fs::write(self.output_dir.join("cursor.theme"), cursor_theme_content)?;
        
        Ok(())
    }
    
    fn install_to_user_icons(&self) -> Result<()> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        
        let user_icons_dir = home_dir.join(".icons").join(&self.theme_name);
        
        if self.output_dir == user_icons_dir {
            return Ok(());
        }
        
        if user_icons_dir.exists() {
            fs::remove_dir_all(&user_icons_dir)?;
        }
        
        fs::create_dir_all(&user_icons_dir)?;
        
        let cursors_src = self.output_dir.join("cursors");
        let cursors_dst = user_icons_dir.join("cursors");
        
        if cursors_src.exists() {
            copy_dir_all(&cursors_src, &cursors_dst)?;
        }
        
        let index_theme_src = self.output_dir.join("index.theme");
        if index_theme_src.exists() {
            fs::copy(&index_theme_src, user_icons_dir.join("index.theme"))?;
        }
        
        let cursor_theme_src = self.output_dir.join("cursor.theme");
        if cursor_theme_src.exists() {
            fs::copy(&cursor_theme_src, user_icons_dir.join("cursor.theme"))?;
        }
        
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else if ty.is_symlink() {
            let target = fs::read_link(entry.path())?;
            unix_fs::symlink(target, dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    
    Ok(())
}
