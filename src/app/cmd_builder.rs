use crate::cmd;
use crate::docker;
use crate::util::paths;
use crate::exec;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use cmd::{Cmd, Repo};
use colored::*;
use easy_error::{bail, Error, ResultExt};
use std::path::Path;

pub fn update_app<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("build")
        .about("Build the provided command (or all commands in the provided directory)")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(
            Arg::with_name("publish")
                .long("publish")
                .short("p")
                .takes_value(true)
                .help("Publish to the provided repo"),
        )
        .arg(
            Arg::with_name("generate")
                .long("generate-repo-yaml")
                .short("y")
                .requires_all(&["publish"])
                .help("Prints a YAML repository with all the built commands"),
        )
        .arg(
            Arg::with_name("COMMAND_OR_DIR")
                .help("The command file to build (or a directory, which is traversed in search of files with the .mcd extension)")
                .required(true)
                .index(1),
        )
}

pub fn handle_build_command<'a>(
    build_options: &ArgMatches<'a>,
    verbose: bool,
) -> Result<(), Error> {
    let path = String::from(build_options.value_of("COMMAND_OR_DIR").unwrap());
    let cmds = cmd::load_commands(vec![path])?;
    return build_command(
        cmds,
        verbose,
        build_options.value_of("publish"),
        build_options.is_present("generate"),
    );
}

fn build_command(
    cmds: Vec<Cmd>,
    verbose: bool,
    publish: Option<&str>,
    print_yaml: bool,
) -> Result<(), Error> {
    let mut built_cmds: Vec<Cmd> = Vec::new();
    for cmd in cmds {
        if let Some(_) = &cmd.command_path {
            let built_cmd = build_on_the_fly_if_necessary(cmd, verbose)?;
            if let Some(repo) = publish {
                let repo_image_name =
                    format!("{}:{}-{}", repo, &built_cmd.registry, &built_cmd.version);
                publish_image(&built_cmd, &repo_image_name)?;
                eprintln!(
                    "Built {} and published to {}",
                    &built_cmd.registry.blue(),
                    repo_image_name.green()
                );
                built_cmds.push(Cmd {
                    registry: String::from(&repo_image_name),
                    ..built_cmd
                });
            } else {
                eprintln!("Built {}", &built_cmd.registry.green());
                built_cmds.push(built_cmd);
            }
        }
    }
    if print_yaml {
        let repo = Repo {
            commands: built_cmds,
        };
        let yaml = serde_yaml::to_string(&repo).context("Failed to serialize repo as YAML")?;
        println!("{}", yaml);
    }
    return Ok(());
}

pub fn build_on_the_fly_if_necessary(cmd: Cmd, verbose: bool) -> Result<Cmd, Error> {
    let registry = if let Some(command_path) = &cmd.command_path {
        let command_path = paths::canonalize_path(command_path);
        let command_path = Path::new(&command_path);
        let context_path = command_path.parent().unwrap();
        let command_name = command_path.file_name().unwrap().to_str().unwrap();

        if cmd.registry == "Dockerfile" {
            // This means the Dockerfile for this file is next to the command.
            // In this case we just run `docker build .` on the folder of the command
            docker::build_image(&command_name, context_path, None, verbose)?
        } else if cmd.registry.starts_with("AlpinePackages") {
            // This means we should build an alpine image on-the-fly that
            // has the provided packages
            let packages = cmd.registry.replace("AlpinePackages", "");
            let extra_commands = format!("RUN apk --no-cache add bash {}", packages.trim());
            docker::build_command_image_from_base(
                &command_name,
                context_path,
                "alpine",
                &extra_commands,
                verbose,
            )?
        } else {
            // This means the command references an existent base Docker image
            // In this case we build a new image based on it, but overwriting the entrypoint
            docker::build_command_image_from_base(
                &command_name,
                context_path,
                &cmd.registry,
                "",
                verbose,
            )?
        }
    } else {
        cmd.registry.clone()
    };

    return Ok(Cmd {
        registry: registry,
        command_path: None, // since the command was built, we remove the path if any
        ..cmd
    });
}

fn publish_image(cmd: &Cmd, repo_image_name: &str) -> Result<(), Error> {
    exec::exec_and_capture_output(
        "docker",
        vec!["tag", &cmd.registry, &repo_image_name],
        Vec::new(),
        false,
        None,
        "",
    )
    .context(format!(
        "Failed to build Docker image for {}",
        repo_image_name
    ))?;

    let result = exec::exec_and_capture_output(
        "docker",
        vec!["push", &repo_image_name],
        Vec::new(),
        false,
        None,
        &format!(
            "Publishing {} as {}",
            &cmd.name.blue(),
            &repo_image_name.green()
        ),
    )
    .context(format!("Failed to publish {}", repo_image_name))?;

    if result.status.code().unwrap_or(-1) != 0 {
        println!("{}", String::from_utf8_lossy(&result.stdout));
        println!("{}", String::from_utf8_lossy(&result.stderr));
        bail!("Failed to publish image {}", repo_image_name);
    }

    return Ok(());
}
