use crate::cmd;
use crate::util::paths;
use cmd::Cmd;
use easy_error::{bail, Error, ResultExt};
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::{collections::HashMap, path::Path};

/// Given a command file path, builds a Cmd instance
pub fn parse_command_file(file_path: &str) -> Result<Cmd, Error> {
    // let cmd_file = std::fs::canonicalize(file_path).unwrap();
    // let cmd_file = cmd_file.as_path();
    let cmd_file = paths::canonalize_path(file_path);
    let cmd_file = Path::new(&cmd_file);

    if !cmd_file.exists() {
        bail!("Command file not found: {}", file_path);
    }

    // let cmd_file = normalize_path(file_path);

    let name = cmd_file
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .replace(".mcd", "");
    let mut version = String::from("0.1.0");
    let mut group = String::new();
    let mut description = String::new();
    let mut registry: Option<String> = None;
    let mut extra: HashMap<String, String> = HashMap::new();
    let mut workdir: Option<String> = None;
    let mut volumes: Vec<String> = Vec::new();
    let mut user = String::new();
    let mut align_with_host_user = true;
    let mut enable_dynamic_volume_mounts = false;
    let mut needs_tty = false;
    let mut needs_ssh = false;

    let file =
        File::open(cmd_file).context(format!("Failed to load command file {}", file_path))?;
    let reader = BufReader::new(file);
    for (idx, line) in reader.lines().enumerate() {
        let line = line.context("Failed to read file")?;
        if !line.starts_with("# @") {
            // ignore non-annotation lines
            continue;
        }

        let parsed_line = parse_line(&line);
        if parsed_line.is_none() {
            bail!("Invalid annotation at line {}: {}", idx, line);
        }

        // process potential annotation
        let (annotation_name, value) = parsed_line.unwrap();
        let annotation_name: &str = &annotation_name;
        match annotation_name {
            "version" => version = value,
            "group" => group = value,
            "description" => description = value,
            "needs_ssh" => needs_ssh = value == "true",
            "needs_tty" => needs_tty = value == "true",
            "align_with_host_user" => align_with_host_user = value == "true",
            "enable_dynamic_volume_mounts" => enable_dynamic_volume_mounts = value == "true",
            "vol" => volumes.push(value),
            "user" => user = value,
            "from" => {
                if value.starts_with("Dockerfile") {
                    let parent_folder = cmd_file.parent().unwrap().to_string_lossy();
                    let dockerfile = format!("{}/{}", parent_folder, value);
                    if !Path::new(&dockerfile).exists() {
                        bail!(
                            "Expected Dockerfile for command {} does not exist at {}",
                            name,
                            dockerfile
                        );
                    }
                    registry = Some(String::from("Dockerfile"));
                } else {
                    registry = Some(value)
                }
            }
            "workdir" => workdir = Some(value),
            _ => {
                extra.insert(annotation_name.to_string(), value);
                ()
            }
        }
    }

    if registry.is_none() {
        // this field is mandatory
        bail!(
            "Did not find mandatory @from annotation in command {}",
            file_path
        );
    }

    let registry = registry.unwrap();
    let command_path = Some(String::from(file_path));

    let cmd = Cmd {
        name,
        group,
        description,
        registry,
        extra,
        workdir,
        volumes,
        version,
        user,
        enable_dynamic_volume_mounts,
        needs_tty,
        needs_ssh,
        command_path,
        align_with_host_user,
    };

    return Ok(cmd);
}

fn parse_line(line: &str) -> Option<(String, String)> {
    let annotation = line.replace("# @", "");
    let mut words = annotation.split_whitespace();
    if let Some(annotation_name) = words.next() {
        let rest: Vec<&str> = words.collect();
        return Some((String::from(annotation_name), String::from(rest.join(" "))));
    } else {
        return None;
    }
}
