use crate::theme;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// NOTE: Here we use serde (toml) since its a config file come on guys
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub document_directories: Vec<String>,
    pub font_name: Option<String>,
    pub theme: theme::Theme,
}

impl Config {
    pub fn new(
        document_base_dir: &std::path::Path,
        config_file: &std::path::Path,
    ) -> Option<Config> {
        let mut config = Config::default();
        config
            .document_directories
            .push(document_base_dir.to_string_lossy().to_string());
        if config_file.exists() {
            let conf_file_content = match std::fs::read_to_string(config_file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "[ERR]: Failed to read config file {}: {e}",
                        config_file.display()
                    );
                    return None;
                }
            };
            config = match toml::de::from_str(&conf_file_content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[ERR]: Failed to parse config: {e}");
                    return None;
                }
            };
            for p in &mut config.document_directories {
                let np = match std::path::PathBuf::from_str(p) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("[ERR]: Failed to parse document directory string {p}: {e}");
                        return None;
                    }
                };
                let mut copy = document_base_dir.to_path_buf();
                copy.push(np);
                *p = copy.to_string_lossy().to_string();
            }
        } else {
            match std::fs::write(
                config_file,
                match toml::ser::to_string_pretty(&config) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[ERR]: Failed to serialize {config:?}: {e}");
                        return None;
                    }
                },
            ) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!(
                        "[ERR]: Failed to write config to {}: {e}",
                        config_file.display()
                    );
                    return None;
                }
            };
        }
        Some(config)
    }
}
