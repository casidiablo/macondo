use easy_error::Terminator;
use macondo::app::core::{execute_command, handle_meta_commands_or_return_cmd, main_app};

fn main() -> Result<(), Terminator> {
    let matches = main_app().get_matches();

    return if let Some(cmd) = handle_meta_commands_or_return_cmd(&matches)? {
        execute_command(cmd, &matches)
    } else {
        Ok(())
    };
}
