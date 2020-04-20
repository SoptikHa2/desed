mod sed;
use sed::debugger::Debugger;
mod cli;
mod ui;
use ui::generic::UiAgent;
use ui::tui::Tui;

fn main() {
    if let Err(error) = run() {
        eprintln!("An error occured: {}", error);
    }
}

fn run() -> Result<(), String> {
    let settings = cli::parse_arguments()?;
    let debugger = Debugger::_mock(settings)?;
    let tui = Tui::new(debugger)?;
    tui.start()
}
