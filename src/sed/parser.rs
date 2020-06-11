use logos::{Lexer, Logos};

/// This parses debug output from sed.
/// It might look like this:
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
/// ---
///
/// Some tricky outputs that we need to handle correctly (notice newlines):
///
/// ```sh
/// SED PROGRAM:
///   /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
///   /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// INPUT:   'kniha.txt' line 1
/// PATTERN: 1. Kapitola
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// 1. Kapitola
/// ===========
///  
/// INPUT:   'kniha.txt' line 2
/// PATTERN: Pomoci teto kapitoly je ukazano,
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// Pomoci teto kapitoly je ukazano,
/// INPUT:   'kniha.txt' line 3
/// PATTERN: jak funguji funkce 1, i, c, d a m.
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// jak funguji funkce 1, i, c, d a m.
/// INPUT:   'kniha.txt' line 4
/// PATTERN: 1.1 Podkapitola 1
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// 1.1 Podkapitola 1
/// -----------------
/// INPUT:   'kniha.txt' line 5
/// PATTERN: Nadpisy jsou rozpoznany podle
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// Nadpisy jsou rozpoznany podle
/// INPUT:   'kniha.txt' line 6
/// PATTERN: cisel na zacatku radku.
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// cisel na zacatku radku.
/// INPUT:   'kniha.txt' line 7
/// PATTERN: 2. Konec
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// 2. Konec
/// ===========
///  
/// INPUT:   'kniha.txt' line 8
/// PATTERN: Doufejme, ze tyto priklady
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// Doufejme, ze tyto priklady
/// INPUT:   'kniha.txt' line 9
/// PATTERN: zdurazni funkcnost funkci
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// zdurazni funkcnost funkci
/// INPUT:   'kniha.txt' line 10
/// PATTERN: a, i, c, d a n.
/// COMMAND: /^[1-9][1-9]*\\.[^1-9]/ a\===========
///  
///
/// COMMAND: /^[1-9][1-9]*\\.*[1-9]/ a\-----------------
///
/// END-OF-CYCLE:
/// a, i, c, d a n.
///
/// ```
pub struct SedDebugOutputParser<'a> {
    sed_output: &'a str,
}
impl<'a> SedDebugOutputParser<'a> {
    pub fn new(output: &'a str) -> SedDebugOutputParser {
        SedDebugOutputParser { sed_output: output }
    }

    /// Parse saved source code and return debug shards
    /// which should be analyzed further.
    pub fn parse(&self) {
        let mut lex: Lexer<DebugShardToken> = DebugShardToken::lexer(self.sed_output);

        while let Some(next_value) = lex.next() {
            let slice = lex.slice();
            eprintln!("{:?}, {}", next_value, slice);
        }
    }
}

fn parse_program_source(lex: &mut Lexer<DebugShardToken>) -> Option<Vec<String>> {
    Some(vec![])
}
fn parse_input_source(lex: &mut Lexer<DebugShardToken>) -> Option<(String, usize)> {
    Some((String::from(""), 0))
}
fn parse_pattern_space(lex: &mut Lexer<DebugShardToken>) -> Option<String> {
    Some(String::from(""))
}
fn parse_hold_space(lex: &mut Lexer<DebugShardToken>) -> Option<String> {
    Some(String::from(""))
}
fn parse_command(lex: &mut Lexer<DebugShardToken>) -> Option<String> {
    Some(String::from(""))
}
fn parse_regex_matches(lex: &mut Lexer<DebugShardToken>) -> Option<Vec<String>> {
    Some(vec![])
}
fn parse_output(lex: &mut Lexer<DebugShardToken>) -> Option<String> {
    Some(String::from(""))
}

/// One instruction sed outputs as debug log.
/// This might be one command, one output,
/// end of cycle instruction, or multiple matched regexes.
#[derive(Logos, Debug, PartialEq)]
enum DebugShardToken {
    /// sed program source
    #[regex("SED PROGRAM:\n[.\n]*", parse_program_source)]
    ProgramSource(Vec<String>),
    /// INPUT: instruction. This specifies
    /// where does sed takes input from in
    /// current cycle.
    ///
    /// Contains filename and line number.
    #[regex("INPUT:   '[^']*' line [0-9]+", parse_input_source)]
    InputSource((String, usize)),
    /// PATTERN: instruction. This specifies
    /// pattern space contents. This
    /// might span multiple lines.
    #[regex("PATTERN: [.\n]*", parse_pattern_space)]
    PatternSpace(String),
    /// HOLD: instruction. This specifies
    /// hold space contents. This
    /// might span multiple lines.
    #[regex("HOLD:    [.\n]*", parse_hold_space)]
    HoldSpace(String),
    /// COMMAND: instruction. This specifies
    /// the instruction sed is currently executing.
    /// This might as well span multiple lines.
    #[regex("COMMAND: [.\n]*", parse_command)]
    Command(String),
    /// MATCHED REGEX REGISTERS instruction. This
    /// contains zero or more regex matches and
    /// does span multiple lines in multiple different
    /// configurations.
    #[regex(
        "MATCHED REGEX REGISTERS(\n  regex\\[[0-9]+\\] = [0-9]+-[0-9]+ '[^']*')*",
        parse_regex_matches
    )]
    RegexMatches(Vec<String>),
    /// END-OF-CYCLE instructions. This marks
    /// end of processing of current input.
    /// New one will be loaded.
    #[token("END-OF-CYCLE:")]
    EndOfCycle,
    /// Output string of sed. This is anything
    /// that begins on a newline.
    #[regex("[.\n]*", parse_output)]
    Output(String),

    #[error]
    Error,
}
