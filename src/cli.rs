use anyhow::{Context, Result};
use clap::{crate_version, App, Arg, ArgMatches};
use std::path::PathBuf;
use std::str::FromStr;

pub fn parse_arguments() -> Result<Options> {
    let matches = App::new("Desed")
        .version(crate_version!())
        .author("Petr Šťastný <desed@soptik.tech>")
        .about("Sed script debugger. Debug and demystify your sed scripts with TUI debugger.")
        .arg(Arg::with_name("sed_n")
            .short('n')
            .long("quiet")
            .long("silent")
            .help("sed: suppress automatic printing of pattern space")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed_E")
            .short('E')
            .long("regexp-extended")
            .help("sed: use extended regular expressions in the script")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed_sandbox")
            .long("sandbox")
            .help("sed: operate in sandbox mode (disable e/r/w commands).")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed_z")
            .long("null-data")
            .short('z')
            .help("sed: separate lines by NUL characters")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("verbose")
            .long("verbose")
            .short('v')
            .help("This will enable various debug printing to stderr.")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed-path")
            .long("sed-path")
            .help("Specify path to sed that should be used. If omitted, gsed/sed from your $PATH will run.")
            .takes_value(true)
            .required(false))
        .arg(Arg::with_name("sed-script")
            .help("Input file with sed script")
            .required(true)
            .multiple(false)
            .index(1))
        .arg(Arg::with_name("input-file")
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
        let sed_script: PathBuf = PathBuf::from_str(matches.value_of("sed-script").unwrap())
            .with_context(|| "Failed to load sed script path")?;
        // UNWRAP: It's safe because we define input-file in the CLI code above, so we are certain it exists.
        let input_file: PathBuf = PathBuf::from_str(matches.value_of("input-file").unwrap())
            .with_context(|| "Failed to load input file path.")?;

        let sed_path: Option<String> = matches.value_of("sed-path").map(String::from);

        let mut sed_parameters: Vec<String> = Vec::with_capacity(4);
        let mut debug = false;

        if matches.is_present("sed_n") {
            sed_parameters.push(String::from("-n"));
        }
        if matches.is_present("sed_E") {
            sed_parameters.push(String::from("-E"));
        }
        if matches.is_present("sed_sandbox") {
            sed_parameters.push(String::from("--sandbox"));
        }
        if matches.is_present("sed_z") {
            sed_parameters.push(String::from("-z"));
        }
        if matches.is_present("verbose") {
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
