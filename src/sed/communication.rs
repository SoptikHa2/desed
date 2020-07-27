use crate::cli::Options;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};

/// This handles communication with GNU sed.
pub struct SedCommunicator {
    options: Options,
}
impl SedCommunicator {
    pub fn new(options: Options) -> Self {
        SedCommunicator { options }
    }

    pub fn get_sed_output(&mut self) -> Result<String> {
        let mut path_to_be_used: &String = &String::from("sed");
        if let Some(path) = &self.options.sed_path {
            path_to_be_used = path;
        }

        let mandatory_parameters = vec![
            "--debug",
            "-f",
            self.options
                .sed_script
                .to_str()
                .with_context(|| format!("Invalid sed script path. Is it valid UTF-8?"))?,
            self.options
                .input_file
                .to_str()
                .with_context(|| format!("Invalid input path. Is it valid UTF-8?"))?,
        ];
        let constructed_cmd_line = self
            .options
            .sed_parameters
            .iter()
            .map(|s| s.as_str())
            .chain(mandatory_parameters.iter().map(|s| *s))
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
                    if  self.options.verbose{ format!("\n[Info] Sed was called using \"{} {}\"", &path_to_be_used, constructed_cmd_line.join(" ")) } else { format!("") }
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
        if self.options.sed_path.is_none() && sed_debug_command.len() == 0 {
            self.options.sed_path = Some(String::from("gsed"));
            if self.options.verbose {
                eprintln!(
                        "[Info] Sed failed and didn't return any output. As sed path wasn't specified, trying again with \"gsed\". If even that won't work, make sure \
                                sed is able to process your script. Most common mistake is forgeting to use -E."
                    );
            }
            return self.get_sed_output();
        }

        Ok(String::from_utf8(sed_debug_command).with_context(|| "String received from sed doesn't seem to be UTF-8. If this continues to happen, please report a bug.")?)
    }
}

