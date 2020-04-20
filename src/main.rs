mod sed;
use sed::debugger::Debugger;
mod cli;
mod ui;

fn main() {
    let options = cli::construct_app();
    println!("{:?}", options);
}
