use clap::{crate_version, App, Arg, ArgMatches};
use std::path::PathBuf;

pub fn construct_app<'a, 'b>() -> Result<Options, String> {
    let matches = App::new("Desed")
        .version(crate_version!())
        .author("Petr Šťastný <petr.stastny01@gmail.com>")
        .about("Sed script debugger. Debug and demystify your sed scripts with TUI debugger.")
        .arg(Arg::with_name("history-limit")
            .long("history-limit")
            .takes_value(true)
            .default_value("1000")
            .help("Desed by default saves execution state history, allowing you to step backwards. However this might cause problems with extremely long files. This option limits maximum number of sed execution states helt in memory. Set to 0 to allow unlimited usage."))
        .arg(Arg::with_name("sed-script")
            .help("Input file with sed script")
            .required(true)
            .multiple(false)
            .index(1))
        .arg(Arg::with_name("input-file")
            .help("File with data for sed to process. Use '-' to read from stdin instead.")
            .required(true)
            .index(2))
        .arg(Arg::with_name("sed-parameters")
            .help("Parameters to be passed to sed. Following options are ignored: -e, -f, -i, --help, --version")
            .index(3)).get_matches();
    Options::from_matches(matches)
}

#[derive(Debug)]
pub struct Options {
    pub history_limit: Option<usize>,
    pub sed_script: PathBuf,
    pub input_file: Option<PathBuf>,
    pub sed_parameters: Vec<String>,
}
impl Options {
    pub fn from_matches(matches: ArgMatches) -> Result<Options, String> {
        Err(String::from("Not implemented"))
    }
}
