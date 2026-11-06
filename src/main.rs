mod sed;
use sed::debugger::Debugger;
mod cli;
use cli::Options;
mod file_watcher;
mod ui;
use anyhow::{anyhow, Result};
use file_watcher::FileWatcher;
use ui::generic::{ApplicationExitReason, UiAgent};
use ui::tui::Tui;

fn main() {
    // If an error occurs, we do not want to clear terminal, it's useful for the error to remain visible.
    // But we want to clear terminal when user just exited GUI normally.
    let mut clear_terminal: bool = true;

    if let Err(error) = run(0) {
        eprintln!("An error occurred: {}", error);
        clear_terminal = false;
    }
    if let Err(error) = Tui::restore_terminal_state(clear_terminal) {
        eprintln!("An error occurred while attempting to reset terminal to previous state. Consider using 'reset' command. Error: {}", error);
    }
}

fn watch_files(settings: &Options) -> Result<FileWatcher> {
    let mut fw = FileWatcher::init()?;

    fw.add_watch(&settings.sed_script)?;
    fw.add_watch(&settings.input_file)?;
    fw.start()?;

    Result::Ok(fw)
}

/// Debug application and start at specified
/// state if possible
fn run(target_state_number: usize) -> Result<()> {
    let settings = cli::parse_arguments()?;
    let watcher = watch_files(&settings)?;
    let debugger = Debugger::run_from(settings)?;

    if debugger.count_of_states() == 0 {
        return Err(anyhow!("Sed didn't run, aborting.\nThis usually means you didn't use GNU sed (didn't install gsed on mac os), or you forgot the -E flag."));
    }

    let tui = Tui::new(&debugger, watcher, target_state_number)?;
    match tui.start()? {
        ApplicationExitReason::UserExit => Ok(()),
        ApplicationExitReason::Reload(instruction_number) => run(instruction_number),
    }
}
