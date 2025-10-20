use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub theme_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("."),
            output_dir: PathBuf::from("./out"),
            theme_name: "Koosh-Generated".into(),
        }
    }
}

