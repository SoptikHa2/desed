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
    Tui::restore_terminal_state();
}

/// Debug application and start at specified
/// state if possible
fn run(target_state_number: usize) -> Result<()> {
    let settings = cli::parse_arguments()?;
    let mut debugger = Debugger::new(settings)?;
    for _ in 0..target_state_number {
        debugger.next_state();
    }
    let tui = Tui::new(debugger)?;
    match tui.start()? {
        ApplicationExitReason::UserExit => {
            return Ok(());
        }
        ApplicationExitReason::Reload(instruction_number) => {
            return run(instruction_number);
        }
    }
}
