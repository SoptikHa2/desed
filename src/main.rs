mod sed;
use sed::debugger::Debugger;
mod cli;
use cli::Options;
mod ui;
mod file_watcher;
use file_watcher::FileWatcher;
use anyhow::Result;
use ui::generic::{ApplicationExitReason, UiAgent};
use ui::tui::Tui;

fn main() {
    if let Err(error) = run(0) {
        eprintln!("An error occured: {}", error);
    }
    if let Err(error) = Tui::restore_terminal_state() {
        eprintln!("An error occured while attempting to reset terminal to previous state. Consider using 'reset' command. Error: {}", error);
    }
}

fn watch_files(settings: &Options) -> Result<FileWatcher> {
    let mut fw = FileWatcher::init()?;

    fw.add_watch(&settings.sed_script);
    fw.add_watch(&settings.input_file);

    return Result::Ok(fw);
}

/// Debug application and start at specified
/// state if possible
fn run(target_state_number: usize) -> Result<()> {
    let settings = cli::parse_arguments()?;
    let watcher = watch_files(&settings)?;
    let debugger = Debugger::new(settings)?;
    let tui = Tui::new(&debugger, watcher, target_state_number)?;
    match tui.start()? {
        ApplicationExitReason::UserExit => {
            return Ok(());
        }
        ApplicationExitReason::Reload(instruction_number) => {
            return run(instruction_number);
        }
    }
}
