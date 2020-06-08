use super::{cmd_builder, repo_management};
use crate::{cmd, config, exec};
use clap::{App, AppSettings, Arg, ArgMatches};
use cmd::{executer, parser, Cmd};
use colored::*;
use easy_error::{bail, Terminator};
use std::{path::Path, process::exit};

pub fn main_app<'a, 'b>() -> App<'a, 'b> {
    return App::new("macondo")
        .about("Macondo... a town of commands")
        .setting(AppSettings::AllowExternalSubcommands)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(
            Arg::with_name("dry_run")
                .short("0")
                .long("dry-run")
                .help("Only prints the command that would be run"),
        )
        .arg(
            Arg::with_name("disable_dynamic_mounts")
                .short("d")
                .long("disable-dynamic-mounts")
                .help("If set will disable the dynamic mounting of volumes"),
        )
        .arg(Arg::with_name("verbose").long("verbose").short("v"))
        .subcommand(cmd_builder::update_app())
        .subcommand(repo_management::repo_management_app());
}

/// Handles any cli subcommand or defaults to return a fully built `Cmd`
pub fn handle_meta_commands_or_return_cmd(app: &ArgMatches) -> Result<Option<Cmd>, Terminator> {
    if let Some(build_options) = app.subcommand_matches("build") {
        cmd_builder::handle_build_command(build_options, app.is_present("verbose"))?;
        return Ok(None);
    }

    if let Some(repo) = app.subcommand_matches("repo") {
        repo_management::handle(repo)?;
        return Ok(None);
    }

    let config = config::load_config()?;
    let cmds = cmd::load_commands(config.repositories)?;

    if app.subcommand().0 == "" {
        eprintln!("{}", "You forgot to provide a command...\n".red());
        cmd::list_commands(&cmds, false);
        return Ok(None);
    }

    ensure_necessary_programs_exist();

    return Ok(Some(resolve_command(app, cmds)?));
}

fn resolve_command(app: &ArgMatches, cmds: Vec<Cmd>) -> Result<Cmd, Terminator> {
    let (command_name, _) = app.subcommand();
    return find_command_in(command_name, cmds);
}

pub fn execute_command(cmd: Cmd, app: &ArgMatches) -> Result<(), Terminator> {
    let (_, args) = app.subcommand();

    let dry_run: bool = app.is_present("dry_run");
    let disable_dynamic_mounts: bool = app.is_present("disable_dynamic_mounts");
    let verbose = app.is_present("verbose");

    let ext_args: Vec<&str> = args
        .unwrap()
        .values_of("")
        .unwrap_or(clap::Values::default())
        .collect();

    return executer::execute_command(cmd, ext_args, dry_run, disable_dynamic_mounts, verbose);
}

fn find_command_in(command_name: &str, cmds: Vec<Cmd>) -> Result<Cmd, Terminator> {
    for c in cmds {
        if c.name == command_name {
            return Ok(c);
        }
    }
    let possible_command_path = Path::new(command_name);
    if possible_command_path.exists() {
        return Ok(parser::parse_command_file(command_name)?);
    }
    bail!("{} command not found", command_name.red());
}

fn ensure_necessary_programs_exist() {
    if !exec::does_command_exist("docker") {
        eprintln!("{} {}", "docker".blue(), "command not found".red());
        println!("Most commands are packaged as Docker images that this tool runs for you.");
        println!("Install docker using your package manager and try again.");
        exit(1);
    }
}
