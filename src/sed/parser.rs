use anyhow::Result;
use crate::sed::debugger::DebuggingState;

/// Parsed anotation received from GNU sed
pub struct SedAnnotation<'a> {
    /// Source code of sed program, cleaned up.
    /// That means one command per line, without comments. This is what gets displayed to user.
    pub program_source: &'a str,
    /// All states sed ever was in. This is generally one state per instruction. Capturing all of it and storing it
    /// allows us to time-travel during debugging.
    pub states: Vec<DebuggingState<'a>>,
    /// Optionally, sed might've printed something after the last instruction. If it was the case, we show it up here. This is
    /// generally output of the sed script as the user sees it.
    pub last_output: Option<&'a str>,
}

pub struct SedAnnotationParser {}
impl SedAnnotationParser {
    pub fn parse_sed_debug_annotation<'a>(input: String) -> Result<SedAnnotation<'a>> {
        unimplemented!();
    }
}

