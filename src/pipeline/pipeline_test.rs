// Integration test for win2xcur -> xcur2png pipeline

#[cfg(test)]
mod tests {
    use crate::pipeline::{win2xcur, xcur2png};
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_full_pipeline_win2xcur_to_xcur2png() {
        // Create temp directories for intermediate and output files
        let temp_dir = tempdir().unwrap();
        let xcur_dir = temp_dir.path().join("xcur");
        let png_dir = temp_dir.path().join("png");
        std::fs::create_dir_all(&xcur_dir).unwrap();
        std::fs::create_dir_all(&png_dir).unwrap();

        // Convert Windows cursor to X11 Xcursor
        let win_cursor_path = Path::new("win2xcur/sample/crosshair.cur");
        if win_cursor_path.exists() {
            let xcur_output = xcur_dir.join("crosshair");

            let options = win2xcur::converter::ConversionOptions::new().with_scale(1.0);

            let result = win2xcur::converter::convert_windows_cursor(
                win_cursor_path,
                &xcur_output,
                &options,
                |msg| {
                    eprintln!("{}", msg);
                },
            );

            assert!(
                result.is_ok(),
                "Failed to convert .cur to Xcursor: {:?}",
                result.err()
            );
            assert!(xcur_output.exists(), "Xcursor file was not created");

            // Extract PNGs from the generated Xcursor
            let extract_options = xcur2png::ExtractOptions::new()
                .with_prefix("crosshair")
                .with_initial_suffix(0)
                .with_config(true);

            let extracted_files =
                xcur2png::extract_to_pngs(&xcur_output, &png_dir, &extract_options);

            assert!(
                extracted_files.is_ok(),
                "Failed to extract PNGs: {:?}",
                extracted_files.err()
            );
            let files = extracted_files.unwrap();
            assert!(!files.is_empty(), "No PNG files were extracted");

            // Verify the first PNG exists and is valid
            let first_png = &files[0];
            assert!(first_png.exists(), "First PNG file does not exist");

            // Try to load the PNG to verify it's valid
            let img_result = image::open(first_png);
            assert!(img_result.is_ok(), "Could not load extracted PNG");

            // Verify config file was created
            let config_path = png_dir.join("crosshair.conf");
            assert!(config_path.exists(), "Config file was not created");

            let config_content = std::fs::read_to_string(&config_path).unwrap();
            assert!(config_content.contains("#size"), "Config missing header");
            assert!(
                config_content.contains("crosshair_000.png"),
                "Config missing PNG reference"
            );

            println!("Pipeline test successful!");
            println!("  - Converted {} to Xcursor", win_cursor_path.display());
            println!("  - Extracted {} PNG files", files.len());
            println!("  - Generated config file");
        } else {
            eprintln!(
                "Warning: Sample cursor file not found at {}",
                win_cursor_path.display()
            );
            eprintln!("Skipping full pipeline test");
        }
    }

    #[test]
    fn test_xcur2png_round_trip() {
        use crate::pipeline::win2xcur::cur::{CursorFrame, CursorImage};
        use crate::pipeline::win2xcur::xcursor_writer;
        use image::{Rgba, RgbaImage};

        let temp_dir = tempdir().unwrap();

        // Create a simple test cursor
        let mut img = RgbaImage::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                let val = ((x + y) * 4) as u8;
                img.put_pixel(x, y, Rgba([val, val, val, 255]));
            }
        }

        let cursor = CursorImage {
            image: img.clone(),
            hotspot: (16, 16),
            nominal_size: 32,
        };

        let frame = CursorFrame {
            images: vec![cursor],
            delay: 100,
        };

        // Write to X11 format
        let x11_data = xcursor_writer::to_x11(&[frame]).unwrap();
        let xcur_path = temp_dir.path().join("test.xcur");
        std::fs::write(&xcur_path, &x11_data).unwrap();

        // Read back and extract
        let xcursor = xcur2png::xcursor_reader::XcursorFile::from_file(&xcur_path).unwrap();
        assert_eq!(xcursor.images.len(), 1);
        assert_eq!(xcursor.images[0].width, 32);
        assert_eq!(xcursor.images[0].height, 32);
        assert_eq!(xcursor.images[0].xhot, 16);
        assert_eq!(xcursor.images[0].yhot, 16);
        assert_eq!(xcursor.images[0].delay, 100);

        // Extract to PNG
        let png_dir = temp_dir.path().join("pngs");
        std::fs::create_dir_all(&png_dir).unwrap();

        let options = xcur2png::ExtractOptions::new()
            .with_prefix("test")
            .with_config(true);

        let files = xcur2png::extract_to_pngs(&xcur_path, &png_dir, &options).unwrap();
        assert_eq!(files.len(), 1);

        // Verify PNG
        let loaded = image::open(&files[0]).unwrap().to_rgba8();
        assert_eq!(loaded.width(), 32);
        assert_eq!(loaded.height(), 32);

        println!("Round-trip test successful!");
    }
}
