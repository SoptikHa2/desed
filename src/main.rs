mod sed;
use sed::debugger::Debugger;
mod cli;
mod ui;
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

/// Debug application and start at specified
/// state if possible
fn run(target_state_number: usize) -> Result<()> {
    let settings = cli::parse_arguments()?;
    let debugger = Debugger::new(settings)?;
    let tui = Tui::new(&debugger, target_state_number)?;
    match tui.start()? {
        ApplicationExitReason::UserExit => {
            return Ok(());
        }
        ApplicationExitReason::Reload(instruction_number) => {
            return run(instruction_number);
        }
    }
}
