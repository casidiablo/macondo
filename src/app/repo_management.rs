use crate::util::cache::{get_from_cache, is_cacheable};
use crate::cmd;
use crate::config;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use colored::*;
use config::MacondoConfig;
use easy_error::{bail, Error};

pub fn repo_management_app<'a, 'b>() -> App<'a, 'b> {
    return SubCommand::with_name("repo")
        .about("Commands repositories management")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::InferSubcommands)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::UnifiedHelpMessage)
        .subcommand(
            SubCommand::with_name("add")
            .setting(AppSettings::ColoredHelp)
            .about("Adds a repository")
            .arg(Arg::with_name("REPO")
            .help("Repository URL (either http, https, directory path, path to repository YAML file)")
            .required(true)
            .index(1))
        )
        .subcommand(
            SubCommand::with_name("remove")
            .setting(AppSettings::ColoredHelp)
            .about("Removes a repository")
            .arg(Arg::with_name("REPO")
            .help("Repository URL (either http, https, directory path, path to repository YAML file)")
            .required(true)
            .index(1))
        )
        .subcommand(
            SubCommand::with_name("list")
            .setting(AppSettings::ColoredHelp)
            .about("List current repositories")
        )
        .subcommand(
            SubCommand::with_name("update")
            .setting(AppSettings::ColoredHelp)
            .about("Refreshes the list of commands provided by the repositories")
        );
}

pub fn handle<'a>(app: &ArgMatches<'a>) -> Result<(), Error> {
    let (subcommand, args) = app.subcommand();
    match subcommand {
        "list" => handle_list(),
        "update" => handle_update(),
        "add" => handle_add(args.unwrap().value_of("REPO").unwrap()),
        "remove" => handle_remove(args.unwrap().value_of("REPO").unwrap()),
        _ => bail!("Repo subcommand unrecognized: {}", subcommand),
    }
}

fn handle_list() -> Result<(), Error> {
    let config = config::load_config()?;
    if config.repositories.is_empty() {
        println!(
            "{} {}",
            "There does not seem to be any repository configured. Add one using ".yellow(),
            "macondo repo add SOME_REPO".green()
        );
    } else {
        println!("\nCurrent repositories:\n");
        for repo in config.repositories {
            println!("{} provides:\n", repo.blue().bold().underline());
            let cmds = cmd::load_commands(vec![repo])?;
            cmd::list_commands(&cmds, true);
        }
    }
    return Ok(());
}

fn handle_update() -> Result<(), Error> {
    let conf = config::load_config()?;
    for repo in conf.repositories {
        if is_cacheable(&repo) {
            println!("Updating remote repository: {}", repo.blue());
            get_from_cache(&repo, true)?;
        }
    }
    return Ok(());
}

fn handle_add(repo: &str) -> Result<(), Error> {
    let conf = config::load_config()?;

    let repo = String::from(repo);
    if !conf.repositories.contains(&repo) {
        // build a new repository list
        let mut new_repositories: Vec<String> = Vec::new();
        new_repositories.extend(conf.repositories);
        new_repositories.push(String::from(&repo));
        // write new configuration
        let new_conf = MacondoConfig {
            repositories: new_repositories,
        };
        config::save_config(&new_conf)?;
        println!("Added new repo {}", &repo.green());

        // if a cacheable resource, try to update it
        if is_cacheable(&repo) {
            println!("Fetching repository...");
            get_from_cache(&repo, true)?;
        }
    } else {
        println!(
            "{} {}",
            repo.blue(),
            "repository is present already".yellow()
        );
    }

    return handle_list();
}

fn handle_remove(repo: &str) -> Result<(), Error> {
    let conf = config::load_config()?;

    let repo = String::from(repo);
    if let Some(pos) = conf.repositories.iter().position(|j| j == &repo) {
        println!("Removing {}", &repo.red());
        let mut new_repositories: Vec<String> = Vec::new();
        new_repositories.extend(conf.repositories);
        new_repositories.remove(pos);
        let new_conf = MacondoConfig {
            repositories: new_repositories,
        };
        return config::save_config(&new_conf);
    } else {
        println!("{}", "Repository not found".yellow());
    }

    handle_list()?;

    return Ok(());
}
