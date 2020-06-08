use crate::util::paths;
use colored::*;
use easy_error::{Error, ResultExt};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::prelude::Write;
use std::path::Path;

pub fn load_config() -> Result<MacondoConfig, Error> {
    // ensure a config file exists
    let config_filename = config_file_path();
    let config_file = Path::new(&config_filename);
    if config_file.is_dir() {
        panic!(
            "Expected {} to be a file, not a directory",
            config_file.display()
        );
    }

    if config_file.exists() {
        let doc =
            fs::read_to_string(config_file).context("Failed to load ~/.macondo config file")?;
        return serde_yaml::from_str(&doc)
            .context("Failed to parse ~/.macondo config file as YAML");
    }

    let config = MacondoConfig {
        repositories: Vec::new(),
    };
    save_config(&config)?;

    let tool_name = app_name();
    println!(
        "Seems this is the first time you run {}\nBefore this tool is useful at all you must add some repositories. Run {} to get started",
        tool_name.blue(),
        format!("{} repo help", tool_name).green()
    );

    return Ok(config);
}

pub fn save_config(config: &MacondoConfig) -> Result<(), Error> {
    let config_filename = config_file_path();
    let config_yaml =
        serde_yaml::to_string(&config).context("Failed to serialize config as YAML")?;
    let mut file =
        File::create(config_filename).context("Failed to create config file ~/.macondo")?;
    return file
        .write_all(config_yaml.as_bytes())
        .context("Failed to write config to ~/.macondo");
}

fn app_name() -> String {
    let tool_name = std::env::current_exe().unwrap();
    let tool_name = tool_name.file_name().unwrap();
    return String::from(tool_name.to_str().unwrap());
}

fn config_file_path() -> String {
    format!("{}/.macondo", paths::home_dir())
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MacondoConfig {
    #[serde(default)]
    pub repositories: Vec<String>,
}
