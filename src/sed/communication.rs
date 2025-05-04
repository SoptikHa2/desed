use super::debugger::DebuggingState;
use crate::cli::Options;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::{Command, Stdio};

/// This handles communication with GNU sed.
pub struct SedCommunicator {
    options: Options,
}
impl SedCommunicator {
    pub fn new(options: Options) -> Self {
        SedCommunicator { options }
    }
    pub fn get_execution_info_from_sed(&mut self) -> Result<DebugInfoFromSed> {
        let output = self.get_sed_output()?;

        let program_source = self.parse_program_source(&output);
        let label_jump_map = self.build_jump_map(&program_source);
        let frames = self.parse_state_frames(&output, &label_jump_map, program_source.len());
        Ok(DebugInfoFromSed {
            program_source,
            states: frames.0,
            last_output: frames.1,
        })
    }
    fn get_sed_output(&mut self) -> Result<String> {
        let mut path_to_be_used: &String = &String::from("sed");
        if let Some(path) = &self.options.sed_path {
            path_to_be_used = path;
        }

        let mandatory_parameters = [
            "--debug",
            "-f",
            self.options
                .sed_script
                .to_str()
                .with_context(|| "Invalid sed script path. Is it valid UTF-8?".to_string())?,
            self.options
                .input_file
                .to_str()
                .with_context(|| "Invalid input path. Is it valid UTF-8?".to_string())?,
        ];
        let constructed_cmd_line = self
            .options
            .sed_parameters
            .iter()
            .map(|s| s.as_str())
            .chain(mandatory_parameters.iter().copied())
            .collect::<Vec<&str>>();
        let sed_debug_command = Command::new(path_to_be_used)
            .args(&constructed_cmd_line)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .ok()
            .with_context(
                || format!("Sed failed to return output. Shouldn't you use -E option? Are you using GNU sed? Is there sed/gsed in $PATH?{}" ,
                    if  self.options.verbose{ format!("\n[Info] Sed was called using \"{} {}\"", &path_to_be_used, constructed_cmd_line.join(" ")) } else { String::new() }
            ))?
            .stdout;

        if self.options.verbose {
            eprintln!(
                "[Info] Called sed with \"{} {}\", which returned {} lines of output.",
                path_to_be_used,
                constructed_cmd_line.join(" "),
                sed_debug_command.len()
            );
        }

        // If sed returned no output (so it failed) and sed
        // path wasn't specified by user,
        // change executing path to "gsed" and try again.
        if self.options.sed_path.is_none() && sed_debug_command.is_empty() {
            self.options.sed_path = Some(String::from("gsed"));
            if self.options.verbose {
                eprintln!(
                        "[Info] Sed failed and didn't return any output. As sed path wasn't specified, trying again with \"gsed\". If even that won't work, make sure \
                                sed is able to process your script. Most common mistake is forgetting to use -E."
                    );
            }
            return self.get_sed_output();
        }

        String::from_utf8(sed_debug_command).with_context(|| "String received from sed doesn't seem to be UTF-8. If this continues to happen, please report a bug.")
    }

    /// Wait for line that looks like "SED PROGRAM:"
    ///
    /// Then, read each line with two spaces up front (remove those spaces) and save each line
    /// into output vector.
    ///
    /// When we meet a line that doesn't start with two spaces, stop reading and return.
    fn parse_program_source(&self, sed_output: &str) -> Vec<String> {
        sed_output
            .lines()
            .skip_while(|line| *line != "SED PROGRAM:")
            .skip(1)
            .take_while(|line| !line.starts_with("INPUT:   '"))
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
    /// COMMAND: =
    /// 1
    /// ```
    /// That was it, that was whole command. Notice the output of the command.
    ///
    /// A segment with multiple commands ends like this:
    /// ```sh
    /// COMMAND: d
    /// END-OF-CYCLE
    /// ```
    /// And another segment begins. Note that we don't differentiate within segments inside the result itself,
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
    /// This returns individual frames *and* output of the last segment of the sed script.
    fn parse_state_frames(
        &self,
        sed_output: &str,
        label_jump_map: &HashMap<String, usize>,
        lines_of_code: usize,
    ) -> (Vec<DebuggingState>, Option<Vec<String>>) {
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
        // If true, we're currently parsing `MATCHED REGEX REGISTERS`, which lasts several lines.
        let mut currently_loading_regex_matches: bool = false;
        // If true, we're currently parsing `MATCHED REGEX REGISTERS`, but one of the regexes spans
        // multiple lines. Keep loading it.
        let mut currently_loading_multiline_regex_match: bool = false;
        // Was any substitution since last command successful?
        let mut substitution_successful: bool = false;

        // TODO: Multiline regexes are not displayed correctly and will fall to output instead. FIXME!!
        for line in lines {
            // If we are trying to parse regexe matches, do so
            if currently_loading_regex_matches {
                if currently_loading_multiline_regex_match {
                    if line.starts_with("  regex[") {
                        // We PROBABLY have new regex now. There is no way to know for sure.
                        // Just carry on.
                        currently_loading_multiline_regex_match = false;
                    } else {
                        let last_regex_idx = regex_registers.len() - 1;
                        regex_registers
                            .get_mut(last_regex_idx)
                            .unwrap()
                            .push_str(line);
                        continue;
                    }
                }
                match line {
                    x if x.starts_with("  ") => {
                        let rest_of_regex: String = String::from(
                            x.chars()
                                .skip_while(|c| *c != '=')
                                .skip(1)
                                .collect::<String>()
                                .trim(),
                        );
                        // If the regex didn't end, start loading it as multiline regex.
                        // We don't have a way to know this for sure, just guessing.
                        if !&rest_of_regex.ends_with("'") {
                            currently_loading_multiline_regex_match = true;
                        }
                        regex_registers.push(rest_of_regex);
                        substitution_successful = true;
                    }
                    _ => {
                        currently_loading_regex_matches = false;
                    }
                }
            }
            // Do not attempt to match traditionally if we are still matching regexes
            if currently_loading_regex_matches {
                continue;
            }
            match line {
                // Do not record INPUT lines, but reset line number, previous command and pattern space.
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
                    sed_line = self.next_line_position(
                        sed_line,
                        current_command,
                        label_jump_map,
                        lines_of_code,
                        substitution_successful,
                    );

                    // Record new command
                    previous_command = Some(String::from(current_command));

                    // Clear old info, such as output
                    previous_output = None;
                    regex_registers = Vec::new();

                    // If the command is t or T, clear substitution_successful
                    if current_command.starts_with("t") || current_command.starts_with("T") {
                        substitution_successful = false;
                    }
                }
                x if x.starts_with("MATCHED REGEX REGISTERS") => {
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
                    substitution_successful = false;
                }
                x => {
                    // Assume this is returned value
                    if let Some(output) = &mut previous_output {
                        output.push(String::from(x));
                    } else {
                        previous_output = Some(Vec::new());
                        previous_output.as_mut().unwrap().push(String::from(x));
                    }
                }
            }
        }

        (result, previous_output)
    }

    /// Guess next command position.
    ///
    /// Try to guess if the current command jumps anywhere. If so,
    /// try to guess where.
    ///
    /// If not, just increment one.
    fn next_line_position(
        &self,
        current_position: usize,
        current_command: &str,
        label_jump_map: &HashMap<String, usize>,
        lines_of_code: usize,
        last_match_successful: bool,
    ) -> usize {
        // Handle jumps
        match current_command {
            // Unconditional jump
            x if x.starts_with("b") => {
                let rest = x[1..].trim();
                if rest.is_empty() {
                    // Jump to end of script
                    lines_of_code
                } else if let Some(target) = label_jump_map.get(rest) {
                    // Jump to target label
                    *target
                } else {
                    // Label not found, just go one line down I guess?
                    current_position + 1
                }
            }
            // Conditional jump
            // Jump only if last substitution was successful
            // (or, in case of T, only if the last substitution was not successful)
            x if x.starts_with("t") | x.starts_with("T") => {
                if (x.starts_with("t") && last_match_successful)
                    || (x.starts_with("T") && !last_match_successful)
                {
                    let rest = x[1..].trim();
                    if rest.is_empty() {
                        // jump to end of script
                        lines_of_code
                    } else if let Some(target) = label_jump_map.get(rest) {
                        // Jump to target label
                        *target
                    } else {
                        // Label not found, just go one line down I guess?
                        current_position + 1
                    }
                } else {
                    current_position + 1
                }
            }
            _ => {
                // Unknown command, just go down
                current_position + 1
            }
        }
    }

    /// Build label jump map
    fn build_jump_map(&self, source_code: &[String]) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for (i, line) in source_code.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with(":") {
                map.insert(String::from(trimmed.trim_start_matches(":")), i);
            }
        }
        map
    }
}

pub struct DebugInfoFromSed {
    pub program_source: Vec<String>,
    pub states: Vec<DebuggingState>,
    pub last_output: Option<Vec<String>>,
}
