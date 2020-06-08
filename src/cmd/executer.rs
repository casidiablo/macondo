use crate::cmd::Cmd;
use crate::app::cmd_builder;
use crate::docker;
use crate::exec;
use easy_error::{Error, ResultExt, Terminator};
use std::fs::create_dir_all;
use std::path::Path;
use std::process::exit;

pub fn execute_command(
    cmd: Cmd,
    args: Vec<&str>,
    dry_run: bool,
    disable_dynamic_mounts: bool,
    verbose: bool,
) -> Result<(), Terminator> {
    let mut cmd = cmd_builder::build_on_the_fly_if_necessary(cmd, verbose)?;
    if cmd.align_with_host_user {
        cmd = align_host_user_if_necessary(cmd, verbose)?;
    }

    let docker_run = docker::build_docker_run(&cmd, args, disable_dynamic_mounts)?;
    let docker_args = docker::docker_run_to_args(&docker_run);
    if dry_run {
        println!("docker {}", docker_args.join(" "));
    } else {
        // create unexistent host volumes if necessary
        ensure_volumes(&cmd, &docker_run)?;

        let exit_code = exec::exec("docker", docker_args, None)?;
        // exit with same status code as executed command
        exit(exit_code);
    }
    return Ok(());
}

fn ensure_volumes(cmd: &Cmd, docker_run: &docker::DockerRun) -> Result<(), Error> {
    for vol in &docker_run.volumes {
        let from = Path::new(&vol.from);
        if !from.exists() {
            if cmd.align_with_host_user {
                // if user alignment is enabled, and there are volumes whose
                // host directory does not exist, create it manually before
                // running docker. This is necessary because by default Docker
                // will create the folder as root.
                create_dir_all(from).context(format!(
                    "Failed to create mount directory {} for {:?}",
                    from.display(),
                    vol
                ))?;
            } else {
                eprint!(
                    "Command {} will try to mount a directory or file that does not exist: {:?}",
                    cmd.name, vol
                )
            }
        }
    }
    return Ok(());
}

/// If cmd.align_with_host_user is set to true, build a new image based on the
// command's one where a clone of the host user is created in the docker container.
fn align_host_user_if_necessary(cmd: Cmd, verbose: bool) -> Result<Cmd, Terminator> {
    let new_command_tag = docker::align_with_host_user(&cmd.registry, verbose)?;
    Ok(Cmd {
        registry: new_command_tag,
        ..cmd
    })
}
