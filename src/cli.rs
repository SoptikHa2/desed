use anyhow::{Context, Result};
use clap::{crate_version, Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;
use std::str::FromStr;

pub fn parse_arguments() -> Result<Options> {
    let matches = Command::new("Desed")
        .version(crate_version!())
        .author("Petr Šťastný <desed@soptik.tech>")
        .about("Sed script debugger. Debug and demystify your sed scripts with TUI debugger.")
        .arg(Arg::new("sed_n")
            .action(ArgAction::SetTrue)
            .short('n')
            .long("quiet")
            .long("silent")
            .help("sed: suppress automatic printing of pattern space")
            .required(false))
        .arg(Arg::new("sed_E")
            .action(ArgAction::SetTrue)
            .short('E')
            .long("regexp-extended")
            .help("sed: use extended regular expressions in the script")
            .required(false))
        .arg(Arg::new("sed_sandbox")
            .action(ArgAction::SetTrue)
            .long("sandbox")
            .help("sed: operate in sandbox mode (disable e/r/w commands).")
            .required(false))
        .arg(Arg::new("sed_z")
            .action(ArgAction::SetTrue)
            .long("null-data")
            .short('z')
            .help("sed: separate lines by NUL characters")
            .required(false))
        .arg(Arg::new("verbose")
            .action(ArgAction::SetTrue)
            .long("verbose")
            .short('v')
            .help("This will enable various debug printing to stderr.")
            .required(false))
        .arg(Arg::new("sed-path")
            .long("sed-path")
            .help("Specify path to sed that should be used. If omitted, gsed/sed from your $PATH will run.")
            .required(false))
        .arg(Arg::new("sed-script")
            .help("Input file with sed script")
            .required(true)
            .index(1))
        .arg(Arg::new("input-file")
            .help("File with data for sed to process.")
            .required(true)
            .index(2))
        .after_help("EXAMPLE:\
            \n\tdesed increment-number.sed test-suite.txt\n\t\tRuns script stored in increment-number.sed with input in test-suite.txt\
            \n\n\tdesed print-matching.sed test-cases.txt -nE\n\t\tRuns script in .sed file with input in .txt file and parameters -n -E to launched sed\n\n\
        CONTROLS:\
            \n\tMouse scroll, j, k, g, G (just as in vi): scroll through file\
            \n\tMouse click, b: toggle breakpoint on target line\
            \n\ts: Step forward\
            \n\ta: step bAckwards\
            \n\tr: run towards end or next breakpoint\
            \n\tR: the same as r, but backwards\
            \n\tl: instantly reload source code and attempt to stay in the same state you were in\
            \n\tq: quit\
            \n\tYou can prefix most commands with numbers, just as in vi.")
        .get_matches();
    Options::from_matches(matches)
}

#[derive(Debug)]
pub struct Options {
    pub sed_script: PathBuf,
    pub input_file: PathBuf,
    pub sed_parameters: Vec<String>,
    pub verbose: bool,
    pub sed_path: Option<String>,
}
impl Options {
    pub fn from_matches(matches: ArgMatches) -> Result<Options> {
        // UNWRAP: It's safe because we define sed-script in the CLI code above, so we are certain it exists.
        let sed_script: PathBuf = PathBuf::from_str(matches.get_one::<String>("sed-script").unwrap())
            .with_context(|| "Failed to load sed script path")?;
        // UNWRAP: It's safe because we define input-file in the CLI code above, so we are certain it exists.
        let input_file: PathBuf = PathBuf::from_str(matches.get_one::<String>("input-file").unwrap())
            .with_context(|| "Failed to load input file path.")?;

        let sed_path: Option<String> = matches.get_one::<String>("sed-path").map(ToOwned::to_owned);

        let mut sed_parameters: Vec<String> = Vec::with_capacity(4);
        let mut debug = false;

        if matches.get_flag("sed_n") {
            sed_parameters.push(String::from("-n"));
        }
        if matches.get_flag("sed_E") {
            sed_parameters.push(String::from("-E"));
        }
        if matches.get_flag("sed_sandbox") {
            sed_parameters.push(String::from("--sandbox"));
        }
        if matches.get_flag("sed_z") {
            sed_parameters.push(String::from("-z"));
        }
        if matches.get_flag("verbose") {
            debug = true;
        }

        Ok(Options {
            sed_script,
            sed_path,
            input_file,
            sed_parameters,
            verbose: debug,
        })
    }
}
