mod volumes;
extern crate serde_json;
use crate::cmd::Cmd;
use crate::util::paths;
use crate::util::paths::expand_path;
use crate::exec;
use easy_error::{bail, Error, ResultExt};
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;
use std::iter::FromIterator;
use std::path::Path;
use tempfile::NamedTempFile;
use tempfile::TempDir;
use users::{get_current_gid, get_current_uid, get_current_username};
use volumes::{get_dynamic_volume_mounts, parse_volume_mounting, DynamicVolumeMount, VolumeMount};
use colored::*;

pub struct DockerRun {
    image_name: String,
    interactive: bool,
    tty: bool,
    user: Option<String>,
    env_vars: Vec<EnvVar>,
    pub volumes: Vec<VolumeMount>,
    workdir: Option<String>,
    args: Vec<String>,
}

struct EnvVar {
    key: String,
    val: String,
}

/// Given a command, build a DockerRun instance which encapsulates
// all the information needed to run the command via Docker
/// This should take care of mounting volumes, reparsing arguments,
/// setting a workdir, adding env variables, etc.
pub fn build_docker_run(
    cmd: &Cmd,
    args: Vec<&str>,
    disable_dynamic_mounts: bool,
) -> Result<DockerRun, Error> {
    let mut volumes: Vec<VolumeMount> = Vec::new();
    // Add volumes explicitly set in the command manifest
    for vol in &cmd.volumes {
        volumes.push(parse_volume_mounting(&vol)?);
    }

    // if dynamic volume mounting is enable, try to figure out if there are
    // paths to mount and adjust the arguments to point to their new full path
    let args = if cmd.enable_dynamic_volume_mounts && !disable_dynamic_mounts {
        let (dynamic_mounts, new_args): (Vec<VolumeMount>, Vec<String>) =
            get_dynamic_volumes_and_new_args(args, &volumes);
        volumes.extend(dynamic_mounts);
        new_args
    } else {
        args.iter().map(|arg| arg.to_string()).collect()
    };

    // Set an appropriate working directory if one is required
    let workdir = if let Some(workdir) = &cmd.workdir {
        let dynamic_mount = infer_workdir(workdir, &volumes);
        if let Some(mount) = dynamic_mount.mount {
            volumes.push(mount);
        }
        Some(dynamic_mount.path_within_mounted_volume)
    } else {
        None
    };

    let mut env_vars: Vec<EnvVar> = Vec::new();

    // if this command needs access to the current ssh (for git clonning, for instance)...
    if cmd.needs_ssh {
        // then forward the system's ssh-agent socket to the container, if possible
        let ssh_auth_sock = env::var("SSH_AUTH_SOCK");
        if let Ok(sock_file) = ssh_auth_sock {
            volumes.push(VolumeMount {
                from: sock_file,
                to: "/ssh-auth-sock".to_string(),
                options: None,
            });

            env_vars.push(EnvVar {
                key: "SSH_AUTH_SOCK".to_string(),
                val: "/ssh-auth-sock".to_string(),
            });
        } else {
            // if we can't forward the ssh socket we should fail...
            eprintln!("{}", "This commands requires access to SSH, most likely to clone a git repo. But the SSH auth socket was not found.".red());
        }
    }

    // Also provide the host user id and group as env vars.
    // This is useful in case the container needs to write
    // files and change the owner to be the host user.
    env_vars.push(EnvVar {
        key: "HOST_USER_ID".to_string(),
        val: get_current_uid().to_string(),
    });
    env_vars.push(EnvVar {
        key: "HOST_GROUP_ID".to_string(),
        val: get_current_gid().to_string(),
    });

    let user = if cmd.user != "" {
        Some(cmd.user.to_string())
    } else {
        None
    };

    return Ok(DockerRun {
        image_name: cmd.registry.to_string(),
        interactive: true,
        tty: cmd.needs_tty,
        user,
        env_vars,
        volumes,
        workdir,
        args,
    });
}

/// Convert a DockerRun instance to a list of command
/// line args for the `docker` command
pub fn docker_run_to_args(docker_run: &DockerRun) -> Vec<String> {
    let mut docker_args: Vec<String> = Vec::new();
    docker_args.push("run".to_string());

    if docker_run.interactive {
        docker_args.push("-i".to_string());
    }

    if docker_run.tty {
        docker_args.push("-t".to_string());
    }

    if let Some(user) = &docker_run.user {
        docker_args.push("--user".to_string());
        docker_args.push(user.to_string());
    }

    // Set all env vars
    for env_var in &docker_run.env_vars {
        docker_args.push("--env".to_string());
        docker_args.push(format!("{}={}", env_var.key, env_var.val));
    }

    // Set all env vars
    for vol in &docker_run.volumes {
        docker_args.push(String::from("--volume"));
        match &vol.options {
            Some(opts) => docker_args.push(format!("{}:{}:{}", &vol.from, &vol.to, opts)),
            None => docker_args.push(format!("{}:{}", &vol.from, &vol.to)),
        }
    }

    if let Some(workdir) = &docker_run.workdir {
        docker_args.push("--workdir".to_string());
        docker_args.push(workdir.to_string());
    }

    // this is the Docker image path
    docker_args.push(docker_run.image_name.clone());

    // append arguments provided to the command if any
    docker_args.extend(docker_run.args.clone());

    return docker_args;
}

fn get_dynamic_volumes_and_new_args(
    args: Vec<&str>,
    existent_vols: &Vec<VolumeMount>,
) -> (Vec<VolumeMount>, Vec<String>) {
    let mut new_args: Vec<String> = Vec::new();
    let mut volumes: HashSet<VolumeMount> = HashSet::new();
    for arg in &args {
        let as_path = Path::new(&arg);
        if as_path.exists() {
            let dynamic_mount = get_dynamic_volume_mounts(arg, existent_vols);
            if let Some(mount) = dynamic_mount.mount {
                volumes.insert(mount);
            }
            new_args.push(dynamic_mount.path_within_mounted_volume);
        } else {
            new_args.push(arg.to_string());
        }
    }
    return (Vec::from_iter(volumes.iter().cloned()), new_args);
}

fn infer_workdir(workdir: &str, existent_mounts: &Vec<VolumeMount>) -> DynamicVolumeMount {
    if workdir.starts_with("~") || workdir.starts_with("PWD") {
        let host_path = expand_path(workdir);

        if !Path::new(&host_path).exists() {
            panic!("The command was configured with a WORKDIR that does not exist in your local filesystem: {}
            This is most likely the command developer's fault.", &host_path);
        }

        if !Path::new(&host_path).is_dir() {
            panic!(
                "The command was configured with a WORKDIR that is not a directory: {}
            This is most likely the command developer's fault.",
                workdir
            );
        }

        get_dynamic_volume_mounts(&host_path, existent_mounts)
    } else {
        DynamicVolumeMount {
            mount: None,
            path_within_mounted_volume: workdir.to_string(),
        }
    }
}

/// Builds a docker image that wraps the provided command
///
/// The image is built using the provided base docker image
/// A default entrypoint is provided that runs "/the_command"
pub fn build_command_image_from_base(
    command_name: &str,
    command_path: &Path,
    base_image: &str,
    extra_commands: &str,
    verbose: bool,
) -> Result<String, Error> {
    let dockerfile_dir = TempDir::new().context("Could not create temp Dockerfile")?;
    let mut dockerfile_path: NamedTempFile =
        NamedTempFile::new_in(&dockerfile_dir).context("Could not create temp Dockerfile")?;
    write_dummy_dockerfile(
        dockerfile_path.as_file_mut(),
        base_image,
        extra_commands,
        command_name,
    )?;
    return build_image(
        command_name,
        command_path,
        Some(&dockerfile_path.path().to_string_lossy()),
        verbose,
    );
}

/// Builds a new Docker image based on the provided one
/// with a user whose username, user id and user group mirrors
/// that of the host user.
pub fn align_with_host_user(image_name: &str, verbose: bool) -> Result<String, Error> {
    let dockerfile_dir = TempDir::new().context("Could not create temp Dockerfile")?;
    let mut dockerfile_path: NamedTempFile =
        NamedTempFile::new_in(&dockerfile_dir).context("Could not create temp Dockerfile")?;
    write_user_alignment_dockerfile(dockerfile_path.as_file_mut(), image_name)?;
    return build_image(
        &format!("{}.aligned", image_name),
        dockerfile_dir.path(),
        Some(&dockerfile_path.path().to_string_lossy()),
        verbose,
    );
}

/// Builds a docker image from the provided context.
/// Tags it with the provided tag and returns the name of the image.
/// If a dockerfile_path is provided, use that as the -f Dockerfile
/// TODO: detect if buildkit is available. If so, use it.
pub fn build_image(
    tag: &str,
    context_path: &Path,
    dockerfile_path: Option<&str>,
    verbose: bool,
) -> Result<String, Error> {
    let dockerfile = if let Some(custom_dockerfile) = dockerfile_path {
        String::from(custom_dockerfile)
    } else {
        let default_dockerfile_path = context_path.join(std::path::Path::new("Dockerfile"));
        let default_dockerfile = default_dockerfile_path.to_string_lossy();
        if !default_dockerfile_path.exists() {
            bail!(
                "Failed to build {} command. No Dockerfile found at {}",
                tag,
                default_dockerfile
            );
        }
        String::from(default_dockerfile)
    };

    let docker_build_args = vec!["build", "-t", tag, "-f", &dockerfile, "."];

    let result = exec::exec_and_capture_output(
        "docker",
        docker_build_args,
        Vec::new(),
        verbose,
        Some(context_path),
        &format!("Building Docker image for {}...", &tag),
    )
    .context("Failed to build docker image")?;

    if result.status.code().unwrap_or(-1) != 0 {
        println!("{}", String::from_utf8_lossy(&result.stdout));
        println!("{}", String::from_utf8_lossy(&result.stderr));
        bail!("Failed to build docker image");
    }

    return Ok(String::from(tag));
}

fn write_dummy_dockerfile(
    dockerfile: &mut File,
    base_image: &str,
    extra_commands: &str,
    command_name: &str,
) -> Result<(), Error> {
    let contents = f!("FROM {base_image}
{extra_commands}
ADD https://raw.githubusercontent.com/Anvil/bash-argsparse/master/argsparse.sh /
RUN chmod a+r /argsparse.sh
COPY {command_name} /{command_name}
RUN chmod a+x /{command_name}
ENTRYPOINT [ \"/{command_name}\" ]
");

    write!(dockerfile, "{}", contents).context("Failed to write temp Dockerfile")?;
    Ok(())
}

fn write_user_alignment_dockerfile(dockerfile: &mut File, base_image: &str) -> Result<(), Error> {
    let user_alignment = include_str!("user-alignment.sh")
        .replace(
            "MACONDO_HOST_USER_ID_PLACEHOLDER",
            &get_current_uid().to_string(),
        )
        .replace(
            "MACONDO_HOST_GROUP_ID_PLACEHOLDER",
            &get_current_gid().to_string(),
        )
        .replace(
            "MACONDO_HOST_USERNAME_PLACEHOLDER",
            &get_current_username().unwrap().into_string().unwrap(),
        )
        .replace("MACONDO_HOST_HOME_DIR_PLACEHOLDER", &paths::home_dir());

    let contents = format!("FROM {}\n{}", base_image, user_alignment);

    write!(dockerfile, "{}", contents).context("Failed to write user alignment Dockerfile")?;
    Ok(())
}
