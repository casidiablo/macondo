use crate::util::paths::expand_path;
use easy_error::{bail, Error};
use std::path;
use std::path::Path;
use std::result::Result;

/// Represents the mounting of a host directory to
/// a target directory in a container.
/// The options allow to specify things like "ro" for read only, etc.
/// The paths in from and to are expected to be absolute.
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct VolumeMount {
    pub from: String,
    pub to: String,
    pub options: Option<String>,
}

// Represents a path to be referenced inside the container, and optionally
// a mount that must exist for such path to be available.
// `mount` being None means the volume is already mounted.
#[derive(Debug)]
pub struct DynamicVolumeMount {
    pub mount: Option<VolumeMount>,
    pub path_within_mounted_volume: String,
}

/// Builds a `VolumeMount` out of a Docker's volume mounting string
/// i.e. a string with the form /some/source/path:/some/destination:options
pub fn parse_volume_mounting(mount: &str) -> Result<VolumeMount, Error> {
    let error_msg = format!(
        "Failed to parse volume mounting '{}'. Expected format: FROM:TO[:OPTIONS]",
        mount
    );
    let mut parts = mount.split(":");

    let from = if let Some(from_volume) = parts.next() {
        expand_path(from_volume)
    } else {
        bail!("{}", &error_msg)
    };

    let to = if let Some(to_volume) = parts.next() {
        expand_path(to_volume)
    } else {
        bail!("{}", &error_msg)
    };

    let options = parts.next().map(|opts| String::from(opts));
    return Ok(VolumeMount { from, to, options });
}

/// Given a path, and a list of existent volume mounts,
/// returns a DynamicVolumeMount containing an optional VolumeMount
/// and the full path of the provided path when mounted in VolumeMount.to
///
/// If any of the `existent_mounts` contains the provided `path`,
/// then no VolumeMount is returned and `DynamicVolumeMount.path_within_mounted_volume`
/// would reference the path in one of the existent volumes.
///
/// Note: the provided path MUST exist in the local filesystem
pub fn get_dynamic_volume_mounts(
    path: &str,
    existent_mounts: &Vec<VolumeMount>,
) -> DynamicVolumeMount {
    let as_path = Path::new(&path);
    let as_path = as_path.canonicalize().unwrap();
    let full_path = as_path.to_string_lossy();

    // Sorting this from most to least specific path
    let mut existent_mounts = existent_mounts.clone();
    existent_mounts.sort_by(|lhs, rhs| {
        lhs.from
            .split(path::MAIN_SEPARATOR)
            .count()
            .cmp(&rhs.from.split(path::MAIN_SEPARATOR).count())
            .reverse()
    });
    // We only take into account directories
    let existent_mounts = existent_mounts
        .iter()
        .filter(|v| Path::new(&v.from).is_dir())
        .collect::<Vec<_>>();

    for mount in existent_mounts {
        let path_with_ending_slash = if mount.from.ends_with(path::MAIN_SEPARATOR) {
            mount.from.clone()
        } else {
            format!("{}{}", &mount.from, path::MAIN_SEPARATOR)
        };

        if &mount.from == &full_path || full_path.starts_with(&path_with_ending_slash) {
            // This existent volume already contains the file.
            // Return a dynamic volume mount with the appropiate information
            return DynamicVolumeMount {
                mount: None,
                path_within_mounted_volume: full_path.replacen(&mount.from, &mount.to, 1),
            };
        }
    }

    // Whe reach here if the volumes that were mounted did not contain already
    // the file that we are trying to reference. In such case, we just mount them.
    let path_parent = if as_path.is_file() {
        // When it is a file, we mount the parent folder
        as_path.parent().unwrap().to_string_lossy()
    } else {
        // When it is a directory, we mount it directly
        as_path.to_string_lossy()
    };

    let to = format!("/_mnt{}", &path_parent);
    let addr = format!("/_mnt{}", &as_path.to_str().unwrap());
    DynamicVolumeMount {
        mount: Some(VolumeMount {
            from: String::from(path_parent),
            to,
            options: None,
        }),
        path_within_mounted_volume: addr,
    }
}
