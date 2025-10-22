use std::fs;
use std::path::Path;

pub fn ensure_dir<P: AsRef<Path>>(p: P) -> std::io::Result<()> {
    if !p.as_ref().exists() {
        fs::create_dir_all(&p)?;
    }
    Ok(())
}
