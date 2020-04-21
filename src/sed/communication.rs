use super::debugger::DebuggingState;
use crate::cli::Options;
use std::process::{Command, Stdio};

/// This handles communication with GNU sed.
pub struct SedCommunicator {
    options: Options,
}
impl SedCommunicator {
    pub fn new(options: Options) -> Self {
        SedCommunicator { options }
    }
    pub fn getExecutionInfoFromSed(&self) -> Result<DebugInfoFromSed, String> {
        let output = self.get_sed_output()?;

        let program_source = self.parse_program_source(&output);
        let frames = self.parse_state_frames(&output);
        return Ok(DebugInfoFromSed {
            program_source,
            states: frames,
        });
    }
    fn get_sed_output(&self) -> Result<String, String> {
        let sed_debug_command = Command::new("sed")
            .args(
                vec![
                    "--debug",
                    "-f",
                    self.options
                        .sed_script
                        .to_str()
                        .ok_or(String::from("Invalid sed script path. Is it valid UTF-8?"))?,
                    self.options
                        .input_file
                        .to_str()
                        .ok_or(String::from("Invalid input path. Is it valid UTF-8?"))?,
                ]
                .iter()
                .map(|s| *s)
                .chain(self.options.sed_parameters.iter().map(|s| s.as_str())),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .ok()
            .ok_or(String::from(
                "Sed failed to process your script. Are you using GNU sed? If so, please report the bug.",
            ))?
            .stdout;

        Ok(String::from_utf8(sed_debug_command).ok().ok_or(String::from("String received from sed doesn't seem to be UTF-8. If this continues to happen, please report the bug."))?)
    }

    /// Wait for line that looks like "SED PROGRAM:"
    ///
    /// Then, read each line with two spaces up front (remove those spaces) and save each line
    /// into output vector.
    ///
    /// When we meet a line that doesn't start with two spaces, stop reading and return.
    fn parse_program_source(&self, sed_output: &String) -> Vec<String> {
        sed_output
            .lines()
            .skip_while(|line| *line != "SED PROGRAM:")
            .skip(1)
            .take_while(|line| line.starts_with("  "))
            .map(|line| String::from(line.trim()))
            .collect()
    }

    /// Parse state frames. They look like this:
    ///
    /// ```sh
    /// INPUT:    'input.txt' line 1
    /// PATTERN: abc
    /// COMMAND: s/a/b/g
    /// MATCHED REGEX REGISTERS
    ///   regex[0] = 0-1 'a'
    /// ```
    /// There might be multiple commands within one input line. The example continues:
    /// ```sh
    /// COMMNAD: =
    /// 1
    /// ```
    /// That was it, that was whole command. Notice the output of the command.
    ///
    /// A segment with multiple commands ends like this:
    /// ```sh
    /// COMMAND: d
    /// END-OF-CYCLE
    /// ```
    /// And another segment begins. Note that we don't differentiate within segments inside the result iself,
    /// but we need to during parsing.
    /// ```sh
    /// INPUT:    'input.txt' line 2
    /// PATTERN: bac
    /// COMMDN: s/a/b/g
    /// (...)
    /// ```
    ///
    /// ---
    ///
    /// List of sed commands that we recognize (this list might be incomplete):
    ///
    /// ```sh
    /// INPUT:   'file.txt' line 1 # Defines where we took the pattern space from
    ///                            # at start of segment. This one is ignored.
    /// PATTERN: abc # Defines pattern space value
    /// HOLD:    def # Defines hold space value (can be empty)
    /// COMMAND: s/a/b/g # Defines currently running command
    /// MATCHED REGEX REGISTERS # Defines matched regex for previous command, including global capture group
    ///   regex[0] = 0-1 'a'
    ///   regex[1] = 0-3 'abc'
    /// END-OF-CYCLE:   # End of segment. This is ignored.
    /// hello           # Value printed to stdout. This tends to come after COMMAND or END-OF-CYCLE.
    /// ```
    ///
    fn parse_state_frames(&self, sed_output: &String) -> Vec<DebuggingState> {
        // First of all, skip the sed program source code.
        let lines = sed_output
            .lines()
            .skip_while(|line| !line.starts_with("INPUT: "));

        // Start parsing
        let mut sed_line: usize = 0; // We need to try to keep track of this ourselves.
                                     // Sed doesn't exactly help with this one.
                                     // All the states will end up here
        let mut result: Vec<DebuggingState> = Vec::new();
        // The most recent pattern buffer
        let mut current_pattern = "";
        // The most recent hold buffer
        let mut current_hold = "";
        // The last command that was executed, if any
        let mut previous_command: Option<String> = None;
        // All matched regexes by previous command
        let mut regex_registers: Vec<String> = Vec::new();
        // If sed printed any output because of last command, what was it
        let mut previous_output = None;
        // If true, we're currently parsing `MATCHED REGEX REGISTERS`, which lasts several lines
        let mut currently_loading_regex_matches: bool = false;

        for line in lines {
            // If we are trying to parse regexe matches, do so
            if currently_loading_regex_matches {
                match line {
                    x if x.starts_with("  ") => {
                        unimplemented!();
                    }
                    x => {
                        currently_loading_regex_matches = false;
                    }
                }
            }
            // Do not attempt to match traditionally if we are still matching regexes
            if currently_loading_regex_matches {
                continue;
            }
            match line {
                // Do not record INPUT lines, but reset line number, previous command and patern space.
                x if x.starts_with("INPUT:") => {
                    sed_line = 0;
                    current_pattern = "";
                    previous_command = None;
                }
                // Save pattern space
                x if x.starts_with("PATTERN:") => {
                    current_pattern = x.trim_start_matches("PATTERN:").trim()
                }
                // Save hold space
                x if x.starts_with("HOLD:") => current_hold = x.trim_start_matches("HOLD:").trim(),
                // When we found a command, push previous debugging state
                x if x.starts_with("COMMAND:") => {
                    let current_command = x.trim_start_matches("COMMAND:").trim();
                    // Push state with the *previous* command and location
                    result.push(DebuggingState {
                        pattern_buffer: String::from(current_pattern),
                        hold_buffer: String::from(current_hold),
                        current_line: sed_line,
                        matched_regex_registers: regex_registers,
                        output: previous_output,
                        sed_command: previous_command,
                    });

                    // Push line number forward
                    sed_line = self.next_line_position(sed_line, current_command);

                    // Record new command
                    previous_command = Some(String::from(current_command));

                    // Clear old info, such as output
                    previous_output = None;
                    regex_registers = Vec::new();
                }
                x if x.starts_with("MATCHED_REGEX_REGISTERS") => {
                    currently_loading_regex_matches = true;
                }
                x if x.starts_with("END-OF-CYCLE:") => {
                    // Push last state, just as if we met next command, but the command was nil
                    result.push(DebuggingState {
                        pattern_buffer: String::from(current_pattern),
                        hold_buffer: String::from(current_hold),
                        current_line: sed_line,
                        matched_regex_registers: regex_registers,
                        output: previous_output,
                        sed_command: previous_command,
                    });

                    // Start at the start again
                    sed_line = 0;

                    // Clear old info, such as output
                    previous_command = None;
                    previous_output = None;
                    regex_registers = Vec::new();
                }
                x => {
                    // Assume this is returned value
                    previous_output = Some(String::from(x));
                }
            }
        }

        result
    }

    /// Guess next command position.
    fn next_line_position(&self, current_position: usize, current_command: &str) -> usize {
        // TODO: Handle jumps
        return current_position + 1;
    }
}

pub struct DebugInfoFromSed {
    pub program_source: Vec<String>,
    pub states: Vec<DebuggingState>,
}
