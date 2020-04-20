mod sed;
use sed::debugger::Debugger;
mod cli;
mod ui;

fn main() {
    let settings = cli::parse_arguments();
    if let Ok(settings) = settings {
    } else {
        eprintln!(
            "An error occured while parsing arguments: {}",
            settings.unwrap_err()
        );
    }
}
