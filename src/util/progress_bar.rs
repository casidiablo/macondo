use indicatif::{ProgressBar, ProgressStyle};

const TICK_SETTINGS: (&str, u64) = ("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ", 80);

/// Return a pre-configured progress bar
pub fn get_progress_bar(length: u64, msg: &str) -> ProgressBar {
    let progressbar_style = ProgressStyle::default_spinner()
        .tick_chars(TICK_SETTINGS.0)
        .template(" {spinner} {msg:<30}");

    let progress_bar = ProgressBar::new(length);
    progress_bar.set_style(progressbar_style);
    progress_bar.enable_steady_tick(TICK_SETTINGS.1);
    progress_bar.set_message(msg);

    progress_bar
}
