use crate::util::paths::ensure_file;
use crate::util::paths::expand_path;
use easy_error::{bail, Error, ResultExt};
use reqwest;
use std::fs::OpenOptions;
use std::io::prelude::Write;
use std::path::Path;

/// Tries to load an HTTP resource and caches it.
/// If the cache file already existed, it will retrieve it as-is,
/// unless `invalidate` is set to true
pub fn get_from_cache(resource: &str, invalidate: bool) -> Result<String, Error> {
    if !is_cacheable(resource) {
        bail!(
            "Can't process {}. Only HTTP and HTTPS resources are cacheable",
            resource
        );
    }

    let cache_file = expand_path(&format!(
        "~/.cache/{}",
        resource
            .replace("/", "_")
            .replace(":", "_")
            .replace(".", "_")
    ));

    if Path::new(&cache_file).exists() && !invalidate {
        // if already cached, just return its path
        return Ok(cache_file);
    }

    // Generic error message
    let err = format!("Failed to create file for {} cache", resource);

    // Try to create file if it does not exist yet
    ensure_file(&cache_file).context(&err)?;

    let contents = reqwest::blocking::get(resource)
        .context(format!("Failed to fetch: {}", resource))?
        .text()
        .context(format!("Failed to fetch: {}", resource))?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&cache_file)
        .context(&err)?;
    file.write_all(contents.as_bytes()).context(&err)?;

    return Ok(cache_file);
}

pub fn is_cacheable(resource: &str) -> bool {
    resource.starts_with("http://") || resource.starts_with("https://")
}
