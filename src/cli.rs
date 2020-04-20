use clap::{crate_version, App, Arg, ArgMatches};
use std::path::PathBuf;
use std::str::FromStr;

pub fn parse_arguments<'a, 'b>() -> Result<Options, String> {
    let matches = App::new("Desed")
        .version(crate_version!())
        .author("Petr Šťastný <petr.stastny01@gmail.com>")
        .about("Sed script debugger. Debug and demystify your sed scripts with TUI debugger.")
        .arg(Arg::with_name("sed_n")
            .short("n")
            .long("quiet")
            .long("silent")
            .help("sed: suppress automatic printing of pattern space")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed_E")
            .short("E")
            .long("regexp-extended")
            .help("sed: use extended regular epxressions in the script")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed_sandbox")
            .long("sandbox")
            .help("sed: operate in sandbox mode (disable e/r/w commands).")
            .takes_value(false)
            .required(false))
        .arg(Arg::with_name("sed_z")
            .long("null-data")
            .short("z")
            .help("sed: separate lines by NUL characters")
            .takes_value(false)
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
        .after_help("EXAMPLE:\n\n\tdesed increment-number.sed test-suite.txt\n\t\tRuns script stored in increment-number.sed with input in test-suite.txt\n\n\tdesed print-matching.sed test-cases.txt -nE\n\t\tRuns script in .sed file with input in .txt file and parameters -n -E to launched sed")
        .get_matches();
    Options::from_matches(matches)
}

#[derive(Debug)]
pub struct Options {
    pub sed_script: PathBuf,
    pub input_file: PathBuf,
    pub sed_parameters: Vec<String>,
}
impl Options {
    pub fn from_matches(matches: ArgMatches) -> Result<Options, String> {
        let sed_script: PathBuf = match PathBuf::from_str(matches.value_of("sed-script").unwrap()) {
            Ok(x) => x,
            Err(_) => return Err(String::from("Failed to load sed script path.")),
        };

        let input_file: PathBuf = match PathBuf::from_str(matches.value_of("input-file").unwrap()) {
            Ok(x) => x,
            Err(_) => return Err(String::from("Failed to load input file path.")),
        };

        let mut sed_parameters: Vec<String> = Vec::with_capacity(4);

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

        Ok(Options {
            sed_script,
            input_file,
            sed_parameters,
        })
    }
}
