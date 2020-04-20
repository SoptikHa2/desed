use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub struct SyntaxHighlighter {
    ps: SyntaxSet,
    ts: ThemeSet,
}
impl SyntaxHighlighter {
    pub fn new() -> SyntaxHighlighter {
        SyntaxHighlighter {
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight_source_code_to_ansi(&self, source: &Vec<String>) -> Vec<String> {
        // TODO: There is no sed syntax
        let syntax = self.ps.find_syntax_by_name("Regular Expression").unwrap();
        let mut h = HighlightLines::new(syntax, &self.ts.themes["base16-ocean.dark"]);
        let mut output = Vec::with_capacity(source.len());
        for line in LinesWithEndings::from(&source.join("\n")) {
            let ranges: Vec<(Style, &str)> = h.highlight(line, &self.ps);
            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
            output.push(escaped);
        }
        output
    }
}
