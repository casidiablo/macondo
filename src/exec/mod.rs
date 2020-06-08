extern crate users;
use crate::util::progress_bar;
use colored::*;
use easy_error::{bail, Error, ResultExt};
use std::env::current_dir;
use std::io;
use std::io::ErrorKind::NotFound;
use std::path::Path;
use std::process::{Command, Output, Stdio};

pub fn exec(program: &str, args: Vec<String>, from_dir: Option<&Path>) -> Result<i32, Error> {
    let mut command = Command::new(program);
    command.args(&args);
    if let Some(dir) = from_dir {
        command.current_dir(dir);
    }

    let result = command
        .spawn()
        .context(format!("Could not spawn: {} {}", program, args.join(" ")))?
        .wait_with_output()
        .context(format!(
            "Error waiting on output of: {} {}",
            program,
            args.join(" ")
        ))?;
    match result.status.code() {
        Some(exit_code) => Ok(exit_code),
        None => bail!("Failed to get exit code for {} {}", program, args.join(" ")),
    }
}

pub fn exec_and_capture_output(
    program: &str,
    args: Vec<&str>,
    env_vars: Vec<(&str, &str)>,
    verbose: bool,
    from_dir: Option<&Path>,
    spinner_bar_message: &str,
) -> io::Result<Output> {
    // Determine from which which directory to run this command
    let current_directory = current_dir().unwrap();
    let run_from_dir = from_dir.or(Some(&current_directory)).unwrap();

    // Format message to display while running the command
    let message = if verbose {
        format!(
            "{} (executing {} {} from {})",
            spinner_bar_message,
            program.blue().bold(),
            args.join(" ").blue(),
            run_from_dir.to_string_lossy().bright_yellow()
        )
    } else {
        String::from(spinner_bar_message)
    };
    let spinner = progress_bar::get_progress_bar(spinner_bar_message.len() as u64, &message);

    if verbose {
        spinner.finish_and_clear();
        eprintln!("{}", message);
    }

    // Run the command without showing any output. Just wait for it to finished.
    let mut command = Command::new(program);
    command.args(&args);
    if !verbose {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
    }
    command.current_dir(run_from_dir);

    for (key, val) in env_vars {
        command.env(key, val);
    }

    let output = command.spawn().unwrap().wait_with_output();
    spinner.finish_and_clear();
    return output;
}

pub fn does_command_exist(command: &str) -> bool {
    let result = Command::new(command)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn();
    if let Err(e) = result {
        if e.kind() == NotFound {
            return false;
        }
    }
    return true;
}
