use easy_error::{bail, Error, ResultExt};
use std::env;
use std::env::current_dir;
use std::fs::{create_dir_all, File};
use std::path::Path;

pub fn expand_path(path: &str) -> String {
    if path.starts_with("PWD") {
        path.replacen("PWD", &current_dir().unwrap().to_str().unwrap(), 1)
    } else {
        path.replace("~", &home_dir())
    }
}

/// Returns the full path of the HOME directory
pub fn home_dir() -> String {
    env::var("HOME").unwrap()
}

/// Expands a potentially relative path to absolute
pub fn canonalize_path(path: &str) -> String {
    let as_path = Path::new(path);
    return if path.contains("/") {
        std::fs::canonicalize(path)
            .unwrap()
            .to_string_lossy()
            .to_string()
    } else {
        current_dir()
            .unwrap()
            .as_path()
            .join(as_path)
            .to_string_lossy()
            .to_string()
    };
}

/// Creates a file in the provided path.
/// It also creates all its directories if they do not yet exist
pub fn ensure_file(filename: &str) -> Result<(), Error> {
    let path = Path::new(&filename);
    if path.is_dir() {
        panic!("Expected {} to be a file, not a directory", path.display());
    }

    let parent = path.parent();
    if let Some(parent) = parent {
        // if it does not exist, try to create it
        if !parent.exists() {
            create_dir_all(parent).context(format!(
                "Failed to create directory {} for {}",
                parent.display(),
                filename
            ))?;
        }
    } else {
        bail!("Could not figure out parent dir of {}", filename);
    }

    if path.exists() {
        return Ok(());
    }

    File::create(filename).context(format!("Failed to create file {}", filename))?;
    return Ok(());
}
