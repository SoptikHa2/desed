mod sed;
use sed::debugger::Debugger;
use sed::debugger::DebuggingState;
mod cli;
mod ui;
use ui::generic::{ApplicationExitReason, UiAgent};
use ui::tui::Tui;

fn main() {
    if let Err(error) = run(0) {
        eprintln!("An error occured: {}", error);
    }
}

/// Debug application and start at specified
/// state if possible
fn run(target_state_number: usize) -> Result<(), String> {
    let settings = cli::parse_arguments()?;
    let debug_option = settings.debug;
    let mut debugger = Debugger::new(settings)?;
    if debug_option {
        debug(debugger);
        Ok(())
    } else {
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
}

fn debug(debugger: Debugger) {
    diagnose_sanity_check(&debugger.state_frames, debugger.source_code.len());
    diagnose_jumps(&debugger.state_frames, &debugger.source_code);
    println!("Diagnosis finished.")
}

/// Run basic sanity checks, whether there is at least some source code there,
/// at least few states and whether there is a line of source code for each
/// distinct line as reported by our heuristisc.
fn diagnose_sanity_check(states: &Vec<DebuggingState>, source_code_length: usize) {
    let mut everything_ok = true;

    if source_code_length == 0 {
        println!("[Sanity check] It looks like the source code of your sed file is empty. Did your forget -E flag? This happens when sed exits with an error.");
        everything_ok = false;
    }

    if states.len() == 0 {
        println!("[Sanity check] It looks like there are no debugging states. Did you forget -E flag? This happens when sed exits with an error.");
        everything_ok = false;
    }

    for state in states.iter().skip(1) {
        let mut expected_line_of_code_at_moment_of_failure: Option<usize> = None;
        let current_line = state.current_line - 1;
        if current_line >= source_code_length {
            eprintln!("[Sanity check] According to our heuristics, we should be at line {}, but that line doesn't exist.", state.current_line);
            expected_line_of_code_at_moment_of_failure = Some(current_line);
        }
        if let Some(expected_line_of_code_at_moment_of_failure) =
            expected_line_of_code_at_moment_of_failure
        {
            everything_ok = false;
            println!("[Sanity check] According to our heuristics, we should be at line {}, but that line doesn't exist. Please report a bug.", expected_line_of_code_at_moment_of_failure);
        }
    }

    if everything_ok {
        println!("[Sanity check] Everything fine ✔");
    }
}

/// Diagnose whether all the jumps we came up with match sed.
/// Sed doesn't tell us which line is it processing, just the command.
/// So we use heuristics to guess which line are we processing.
/// This will check if it matches the command sed is actually processing.
fn diagnose_jumps(states: &Vec<DebuggingState>, source_code: &Vec<String>) {
    let mut first_error: Option<(usize, &DebuggingState, &String)> = None;
    for (i, state) in states.iter().skip(1).enumerate() {
        if let Some(sed_command) = &state.sed_command {
            let current_line = state.current_line - 1;
            if let Some(source_code_line) = source_code.get(current_line) {
                if source_code_line.trim() != sed_command.trim() {
                    if first_error.is_none() {
                        first_error = Some((i, state, source_code_line));
                    }
                    eprintln!(
                        "[Jump prediction] Error at state {}: {:?}. Actual LOC: {}",
                        i, state, source_code_line
                    );
                } else {
                    eprintln!(
                        "[Jump prediction] Cannot check: unknown source code at line {}",
                        i
                    );
                }
            }
        }
    }

    if let Some(first_error) = first_error {
        println!(
            "[Jump prediction] Failed. Please submit a bug report. First state with error: {}, {:?}. Actual line: {}",
            first_error.0, first_error.1, first_error.2
        );
    } else {
        println!("[Jump predicion] Everything fine ✔");
    }
}
