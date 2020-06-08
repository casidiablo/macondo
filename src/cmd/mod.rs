extern crate colored;
use crate::util::cache::{get_from_cache, is_cacheable};
use crate::util::paths::expand_path;
use colored::*;
use easy_error::{Error, ResultExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::{fs, path::Path};
use walkdir::WalkDir;

pub mod parser;
pub mod executer;

/// Loads all the commands provided by the manifests listed in ~/.macondo
pub fn load_commands(files: Vec<String>) -> Result<Vec<Cmd>, Error> {
    let mut all_cmds = HashMap::new();
    for resource in files {
        let file: String = if is_cacheable(&resource) {
            get_from_cache(&resource, false)?
        } else {
            expand_path(&resource)
        };
        let file_path = Path::new(&file);
        let cmds = if file_path.is_dir() {
            // A manifest file can point to a directory, i which case
            // it is traversed in search of .mcd files which can be parsed as commands
            load_commands_from_directory(file_path)
        } else if file.ends_with(".mcd") {
            Ok(vec![parser::parse_command_file(&file)?])
        } else {
            load_commands_from_manifest(&file)
        };
        for cmd in cmds? {
            if let Some(previous_cmd) = all_cmds.insert(cmd.name.clone(), cmd) {
                eprintln!(
                    "{}: command {} was overwritten by {}",
                    "Warning".yellow(),
                    &previous_cmd.name,
                    &resource.blue()
                );
            }
        }
    }
    return Ok(all_cmds.values().cloned().collect());
}

fn load_commands_from_directory(dir: &Path) -> Result<Vec<Cmd>, Error> {
    let mut cmds = Vec::new();
    let files = find_all_command_files(dir);
    for file in files {
        cmds.push(parser::parse_command_file(&file)?);
    }
    return Ok(cmds);
}

fn find_all_command_files(dir: &Path) -> Vec<String> {
    return WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().to_str().unwrap().ends_with(".mcd"))
        .map(|e| String::from(e.path().to_str().unwrap()))
        .collect();
}

fn load_commands_from_manifest(manifest: &str) -> Result<Vec<Cmd>, Error> {
    let mut cmds = Vec::new();
    let doc =
        fs::read_to_string(&manifest).context(format!("Could not load manifest {}", &manifest))?;
    let repo: Repo = serde_yaml::from_str(&doc).context(format!(
        "Could not parse manifest {}. Make sure it is a valid yaml document.",
        &manifest
    ))?;
    for cmd in repo.commands {
        cmds.push(Cmd {
            registry: cmd.registry.replace("${version}", &cmd.version),
            ..cmd
        });
    }
    return Ok(cmds);
}

/// Prints all available commands, grouping them according to their group key
pub fn list_commands(commands: &Vec<Cmd>, show_version: bool) {
    let mut groups = HashMap::new();
    for cmd in commands {
        let group_name = if cmd.group != "" { &cmd.group } else { "Other" };
        groups.entry(group_name).or_insert(Vec::new()).push((
            &cmd.name,
            &cmd.version,
            &cmd.description,
        ));
    }

    for (k, v) in &groups {
        if *k == "Other" {
            println!("General commands:\n");
        } else {
            println!("{} commands:\n", k);
        }
        for (cmd, version, description) in v {
            if show_version {
                println!(
                    "  {: <25}{: <10}{}",
                    cmd.green(),
                    version.blue(),
                    description
                );
            } else {
                println!("  {: <25}{}", cmd.green(), description);
            }
        }
        println!();
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Repo {
    pub commands: Vec<Cmd>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cmd {
    // Command metadata fields
    pub name: String,
    #[serde(default)]
    pub group: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,

    // Docker specific fields
    pub registry: String,
    pub volumes: Vec<String>,
    #[serde(default)]
    pub user: String, // TODO use option too
    pub workdir: Option<String>,
    #[serde(default)]
    pub enable_dynamic_volume_mounts: bool,
    #[serde(default)]
    pub needs_tty: bool,

    // Special fields
    #[serde(default)]
    pub needs_ssh: bool,
    // Set to the path of the macondo command in the local filesystem
    // When present, it means this command is not published to any
    // repository, and thus must be built into a Docker image on-the-fly
    pub command_path: Option<String>,
    // If true, create a user in the container matching the host's username,
    // user id and group id.
    //
    // It also sets the Docker USER to that username. But keep in mind that
    // this can be overwritten by the `user` field, which is passed to docker
    // using the `--user` flag which has a higher precedence.
    #[serde(default = "to_true")]
    pub align_with_host_user: bool,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

impl fmt::Display for Cmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn to_true() -> bool {
    true
}

fn default_version() -> String {
    String::from("0.1.0")
}
