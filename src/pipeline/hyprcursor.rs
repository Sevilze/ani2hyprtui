use anyhow::{Context, Result, anyhow};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::pipeline::xcur2png::extractor::{ExtractOptions, extract_to_pngs};

#[derive(Debug, Clone)]
struct HyprManifest {
    name: String,
    description: String,
    version: String,
    cursors_directory: String,
}

impl HyprManifest {
    fn log_info<F>(&self, mut log_fn: F)
    where
        F: FnMut(String),
    {
        log_fn(format!(
            "Manifest: {} v{} - {}",
            self.name, self.version, self.description
        ));
    }
}

#[derive(Debug, Clone)]
struct HyprShape {
    directory: String,
    hotspot_x: f32,
    hotspot_y: f32,
    resize_algorithm: String,
    images: Vec<HyprImage>,
    overrides: Vec<String>,
}

impl HyprShape {
    fn validate<F>(&self, mut log_fn: F) -> Result<()>
    where
        F: FnMut(String),
    {
        if self.directory.is_empty() {
            return Err(anyhow!("Shape directory cannot be empty"));
        }
        for img in &self.images {
            if img.size == 0 {
                return Err(anyhow!("Image {} has invalid size 0", img.file));
            }
            // Warn if delay is extremely low but not 0
            if img.delay > 0 && img.delay < 10 {
                log_fn(format!(
                    "Warning: Very short delay ({}ms) for image {}",
                    img.delay, img.file
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct HyprImage {
    file: String,
    size: u32,
    delay: u32,
}

#[derive(Debug, Clone)]
struct XConfigEntry {
    size: u32,
    hotspot_x: u32,
    hotspot_y: u32,
    image: String,
    delay: u32,
}

pub fn create_cursor_theme<F>(
    input_dir: &Path,
    output_dir: Option<&Path>,
    exact_output: bool,
    mut log_fn: F,
) -> Result<()>
where
    F: FnMut(String) + Copy,
{
    let input_path = input_dir.canonicalize().context("Invalid input path")?;

    // parse manifest
    let manifest_path_hl = input_path.join("manifest.hl");
    let manifest_path_toml = input_path.join("manifest.toml");

    let (manifest, manifest_file_name) = if manifest_path_hl.exists() {
        (parse_manifest_hl(&manifest_path_hl)?, "manifest.hl")
    } else if manifest_path_toml.exists() {
        (parse_manifest_toml(&manifest_path_toml)?, "manifest.toml")
    } else {
        return Err(anyhow!(
            "No manifest.hl or manifest.toml found in input directory"
        ));
    };

    manifest.log_info(log_fn);

    // determine output directory
    let out_path = if let Some(out) = output_dir {
        if exact_output {
            out.to_path_buf()
        } else {
            out.join(format!("theme_{}", manifest.name.replace(" ", "_")))
        }
    } else {
        let parent = input_path.parent().unwrap_or(&input_path);
        parent.join(format!("theme_{}", manifest.name.replace(" ", "_")))
    };

    if out_path.exists() {
        if !exact_output {
            log_fn(format!("Output directory {:?} exists. Cleaning...", out_path));
            fs::remove_dir_all(&out_path)?;
            fs::create_dir_all(&out_path)?;
        } else {
            fs::create_dir_all(&out_path)?;
        }
    } else {
        fs::create_dir_all(&out_path)?;
    }

    // copy manifest
    fs::copy(
        input_path.join(manifest_file_name),
        out_path.join(manifest_file_name),
    )?;

    // process cursors
    let cursors_subdir = &manifest.cursors_directory;
    let cursors_src_dir = input_path.join(cursors_subdir);
    let cursors_out_dir = out_path.join(cursors_subdir);

    if !cursors_src_dir.exists() {
        return Err(anyhow!(
            "Cursors directory {:?} does not exist",
            cursors_src_dir
        ));
    }
    fs::create_dir_all(&cursors_out_dir)?;

    for entry in fs::read_dir(&cursors_src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_str().unwrap().to_string();

            // Check for valid name (alphanumeric + _ - .)
            if !dir_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
            {
                log_fn(format!("Skipping invalid directory name: {}", dir_name));
                continue;
            }

            process_shape(&path, &cursors_out_dir, &dir_name, log_fn)?;
        }
    }

    log_fn(format!("Theme created at {:?}", out_path));
    Ok(())
}

pub fn process_shape<F>(shape_dir: &Path, out_dir: &Path, shape_name: &str, mut log_fn: F) -> Result<()>
where
    F: FnMut(String),
{
    // Parse meta
    let meta_path_hl = shape_dir.join("meta.hl");
    let meta_path_toml = shape_dir.join("meta.toml");

    let (meta_path, meta_file_name) = if meta_path_hl.exists() {
        (meta_path_hl, "meta.hl")
    } else if meta_path_toml.exists() {
        (meta_path_toml, "meta.toml")
    } else {
        return Err(anyhow!("No meta file found in {:?}", shape_dir));
    };

    let shape = if meta_file_name.ends_with(".hl") {
        parse_meta_hl(&meta_path, shape_name)?
    } else {
        parse_meta_toml(&meta_path, shape_name)?
    };

    shape.validate(&mut log_fn)?;

    // Validate images
    for img in &shape.images {
        let img_path = shape_dir.join(&img.file);
        if !img_path.exists() {
            return Err(anyhow!(
                "Image {} missing for shape {}",
                img.file,
                shape_name
            ));
        }
    }

    if shape.images.is_empty() {
        return Err(anyhow!("No images defined for shape {}", shape_name));
    }

    // Create .hlc zip
    let zip_path = out_dir.join(format!("{}.hlc", shape_name));
    let file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // Add meta file
    zip.start_file(meta_file_name, options)?;
    let meta_content = fs::read(&meta_path)?;
    zip.write_all(&meta_content)?;

    // Add images
    for img in &shape.images {
        zip.start_file(&img.file, options)?;
        let img_content = fs::read(shape_dir.join(&img.file))?;
        zip.write_all(&img_content)?;
    }

    zip.finish()?;
    log_fn(format!("Created {}.hlc", shape_name));

    Ok(())
}

pub fn extract_xcursor_to_hypr_source(
    xcursor_path: &Path,
    output_dir: &Path,
    resize_algo: Option<&str>,
    overrides: Vec<String>,
) -> Result<()> {
    let stem = xcursor_path
        .file_stem()
        .ok_or_else(|| anyhow!("Invalid xcursor path: missing filename"))?
        .to_string_lossy()
        .to_string();
    let shape_dir = output_dir.join(&stem);
    fs::create_dir_all(&shape_dir)?;

    let options = ExtractOptions::new().with_prefix(&stem).with_config(true);
    extract_to_pngs(xcursor_path, &shape_dir, &options)?;

    let config_path = shape_dir.join(format!("{}.conf", stem));
    if !config_path.exists() {
        return Err(anyhow!("No config generated for {}", stem));
    }

    let entries = parse_xconfig(&config_path)?;
    if entries.is_empty() {
        return Err(anyhow!("Empty config for {}", stem));
    }

    let meta_path = shape_dir.join("meta.hl");
    let mut meta_file = File::create(meta_path)?;

    let algo = resize_algo.unwrap_or("none");
    writeln!(meta_file, "resize_algorithm = {}", algo)?;

    let first = &entries[0];
    if first.size > 0 {
        writeln!(
            meta_file,
            "hotspot_x = {:.2}",
            first.hotspot_x as f32 / first.size as f32
        )?;
        writeln!(
            meta_file,
            "hotspot_y = {:.2}",
            first.hotspot_y as f32 / first.size as f32
        )?;
    } else {
        writeln!(meta_file, "hotspot_x = 0.0")?;
        writeln!(meta_file, "hotspot_y = 0.0")?;
    }
    writeln!(meta_file, "")?;

    for entry in &entries {
        let file_name = Path::new(&entry.image)
            .file_name()
            .ok_or_else(|| anyhow!("Invalid image path: {}", entry.image))?
            .to_string_lossy();
        writeln!(
            meta_file,
            "define_size = {}, {}, {}",
            entry.size, file_name, entry.delay
        )?;
    }
    writeln!(meta_file, "")?;

    for ov in overrides {
        writeln!(meta_file, "define_override = {}", ov)?;
    }

    fs::remove_file(config_path)?;
    Ok(())
}

pub fn extract_xcursor_theme<F>(
    input_path: &Path,
    output_dir: Option<&Path>,
    resize_algo: Option<&str>,
    exact_output: bool,
    mut log_fn: F,
) -> Result<()>
where
    F: FnMut(String),
{
    let input_path = input_path.canonicalize().context("Invalid input path")?;

    let cursors_path = input_path.join("cursors");
    if !cursors_path.exists() {
        return Err(anyhow!(
            "Input path does not look like an xcursor theme (missing 'cursors' subdir)"
        ));
    }

    let theme_name = input_path
        .file_name()
        .ok_or_else(|| anyhow!("Invalid input path: missing directory name"))?
        .to_string_lossy();

    let out_dir = if let Some(out) = output_dir {
        if exact_output {
            out.to_path_buf()
        } else {
            out.join(format!("extracted_{}", theme_name))
        }
    } else {
        let parent = input_path.parent().unwrap_or(&input_path);
        parent.join(format!("extracted_{}", theme_name))
    };

    if out_dir.exists() {
        if !exact_output {
            log_fn(format!("Output directory {:?} exists. Cleaning...", out_dir));
            fs::remove_dir_all(&out_dir)?;
            fs::create_dir_all(&out_dir)?;
        } else {
            fs::create_dir_all(&out_dir)?;
        }
    } else {
        fs::create_dir_all(&out_dir)?;
    }

    // Write Manifest
    let manifest_path = out_dir.join("manifest.hl");
    let mut manifest_file = File::create(manifest_path)?;
    writeln!(manifest_file, "name = {}", theme_name)?;
    writeln!(
        manifest_file,
        "description = Automatically extracted with ani2hyprtui"
    )?;
    writeln!(manifest_file, "version = 1.0")?;
    writeln!(manifest_file, "cursors_directory = hyprcursors")?;

    let hyprcursors_dir = out_dir.join("hyprcursors");
    fs::create_dir_all(&hyprcursors_dir)?;

    for entry in fs::read_dir(&cursors_path)? {
        let entry = entry?;
        let path = entry.path();

        // Skip symlinks initially, we handle them via overrides later
        if path.is_symlink() || !path.is_file() {
            continue;
        }

        let stem = path
            .file_stem()
            .ok_or_else(|| anyhow!("Invalid cursor filename"))?
            .to_string_lossy()
            .to_string();
        log_fn(format!("Processing {}", stem));

        let shape_dir = hyprcursors_dir.join(&stem);
        fs::create_dir_all(&shape_dir)?;

        // extract using xcur2png logic
        let options = ExtractOptions::new().with_prefix(&stem).with_config(true);

        extract_to_pngs(&path, &shape_dir, &options)?;

        // read the generated config to build meta.hl
        let config_path = shape_dir.join(format!("{}.conf", stem));
        if !config_path.exists() {
            log_fn(format!("Warning: No config generated for {}", stem));
            continue;
        }

        let entries = parse_xconfig(&config_path)?;
        if entries.is_empty() {
            log_fn(format!("Warning: Empty config for {}", stem));
            continue;
        }

        // Generate meta.hl
        let meta_path = shape_dir.join("meta.hl");
        let mut meta_file = File::create(meta_path)?;

        let algo = resize_algo.unwrap_or("none");
        writeln!(meta_file, "resize_algorithm = {}", algo)?;

        // Calculate relative hotspot from the first entry
        let first = &entries[0];
        if first.size > 0 {
            writeln!(
                meta_file,
                "hotspot_x = {:.2}",
                first.hotspot_x as f32 / first.size as f32
            )?;
            writeln!(
                meta_file,
                "hotspot_y = {:.2}",
                first.hotspot_y as f32 / first.size as f32
            )?;
        } else {
            writeln!(meta_file, "hotspot_x = 0.0")?;
            writeln!(meta_file, "hotspot_y = 0.0")?;
        }
        writeln!(meta_file, "")?;

        for entry in &entries {
            let file_name = Path::new(&entry.image)
                .file_name()
                .ok_or_else(|| anyhow!("Invalid image path: {}", entry.image))?
                .to_string_lossy();
            writeln!(
                meta_file,
                "define_size = {}, {}, {}",
                entry.size, file_name, entry.delay
            )?;
        }
        writeln!(meta_file, "")?;

        // Find symlinks pointing to this file
        for sub_entry in fs::read_dir(&cursors_path)? {
            let sub_entry = sub_entry?;
            let sub_path = sub_entry.path();
            if sub_path.is_symlink() {
                if let (Ok(p1), Ok(p2)) = (fs::canonicalize(&path), fs::canonicalize(&sub_path)) {
                    if p1 == p2 {
                        let sym_name = sub_path
                            .file_stem()
                            .ok_or_else(|| anyhow!("Invalid symlink filename"))?
                            .to_string_lossy();
                        writeln!(meta_file, "define_override = {}", sym_name)?;
                    }
                }
            }
        }

        fs::remove_file(config_path)?;
    }

    log_fn(format!("Extracted to {:?}", out_dir));
    Ok(())
}

fn parse_manifest_hl(path: &Path) -> Result<HyprManifest> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut name = String::new();
    let mut description = String::new();
    let mut version = String::new();
    let mut cursors_directory = String::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "name" => name = val.to_string(),
                "description" => description = val.to_string(),
                "version" => version = val.to_string(),
                "cursors_directory" => cursors_directory = val.to_string(),
                _ => {}
            }
        }
    }

    Ok(HyprManifest {
        name,
        description,
        version,
        cursors_directory,
    })
}

fn parse_manifest_toml(path: &Path) -> Result<HyprManifest> {
    let content = fs::read_to_string(path)?;
    let value = content.parse::<toml::Table>()?;

    Ok(HyprManifest {
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        description: value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        version: value
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        cursors_directory: value
            .get("cursors_directory")
            .and_then(|v| v.as_str())
            .unwrap_or("cursors")
            .to_string(),
    })
}

fn parse_meta_hl(path: &Path, shape_name: &str) -> Result<HyprShape> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut shape = HyprShape {
        directory: shape_name.to_string(),
        hotspot_x: 0.0,
        hotspot_y: 0.0,
        resize_algorithm: "none".to_string(),
        images: Vec::new(),
        overrides: Vec::new(),
    };

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "hotspot_x" => shape.hotspot_x = val.parse().unwrap_or(0.0),
                "hotspot_y" => shape.hotspot_y = val.parse().unwrap_or(0.0),
                "resize_algorithm" => shape.resize_algorithm = val.to_string(),
                "define_size" => {
                    // val = size, file, delay
                    let parts: Vec<&str> = val.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        shape.images.push(HyprImage {
                            size: parts[0].parse().unwrap_or(0),
                            file: parts[1].to_string(),
                            delay: parts[2].parse().unwrap_or(0),
                        });
                    }
                }
                "define_override" => shape.overrides.push(val.to_string()),
                _ => {}
            }
        }
    }

    Ok(shape)
}

fn parse_meta_toml(path: &Path, shape_name: &str) -> Result<HyprShape> {
    let content = fs::read_to_string(path)?;
    let table = content.parse::<toml::Table>()?;

    let mut shape = HyprShape {
        directory: shape_name.to_string(),
        hotspot_x: table
            .get("hotspot_x")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32,
        hotspot_y: table
            .get("hotspot_y")
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32,
        resize_algorithm: table
            .get("resize_algorithm")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_string(),
        images: Vec::new(),
        overrides: Vec::new(),
    };

    if let Some(sizes) = table.get("sizes").and_then(|v| v.as_array()) {
        for size_entry in sizes {
            if let Some(entry) = size_entry.as_table() {
                shape.images.push(HyprImage {
                    size: entry.get("size").and_then(|v| v.as_integer()).unwrap_or(0) as u32,
                    file: entry
                        .get("file")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    delay: entry.get("delay").and_then(|v| v.as_integer()).unwrap_or(0) as u32,
                });
            }
        }
    }

    if let Some(overrides) = table.get("overrides").and_then(|v| v.as_array()) {
        for ov in overrides {
            if let Some(s) = ov.as_str() {
                shape.overrides.push(s.to_string());
            }
        }
    }

    Ok(shape)
}

fn parse_xconfig(path: &Path) -> Result<Vec<XConfigEntry>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Format: size xhot yhot filename delay
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 5 {
            entries.push(XConfigEntry {
                size: parts[0].parse().unwrap_or(0),
                hotspot_x: parts[1].parse().unwrap_or(0),
                hotspot_y: parts[2].parse().unwrap_or(0),
                image: parts[3].to_string(),
                delay: parts[4].parse().unwrap_or(0),
            });
        }
    }

    Ok(entries)
}
