use crate::cli::Options;
use crate::sed::parser::{SedAnnotation, SedAnnotationParser};
use crate::sed::communication::SedCommunicator;
use anyhow::Result;

/// Sed program debugger.
///
/// This stores debug states, allowing user
/// to return to previous states of execution.
///
/// This will panic if something bad happens
/// while executing sed, sed isn't the GNU version,
/// or an invalid inner state (which should never happen) happens.
pub struct Debugger<'a> {
    /// Sed source code, one instruction per line.
    ///
    /// If there were multiple instructions on a single line in original source code,
    /// they are spread out so one is on each line.
    pub source_code: &'a str,
    /// Previously visited debugging states, inclding the current one.
    state_frames: Vec<DebuggingState<'a>>,
}

impl<'a> Debugger<'a> {
    /// Create new instance of debugger and launch sed.
    pub fn new(settings: Options) -> Result<Self> {
        let mut communicator = SedCommunicator::new(settings);
        let data: SedAnnotation = SedAnnotationParser::parse_sed_debug_annotation(communicator.get_sed_output()?)?;
        // Shift all pattern matches one frame earlier.
        // The way it's done now (output appears one frame after it's source)
        // is, while the way sed works, very confusing.
        let mut states: Vec<DebuggingState> = data.states;
        states.reverse();
        let mut states_shifted: Vec<DebuggingState> = Vec::with_capacity(states.len());
        let mut previous_output: Option<&str> = data.last_output;
        let mut previous_matches: Vec<String> = Vec::new();
        for state in states {
            states_shifted.push(DebuggingState {
                pattern_buffer: state.pattern_buffer,
                current_line: state.current_line,
                hold_buffer: state.hold_buffer,
                matched_regex_registers: previous_matches,
                output: previous_output,
                sed_command: state.sed_command,
            });
            previous_output = state.output;
            previous_matches = state.matched_regex_registers;
        }
        states_shifted.reverse();
        Ok(Debugger {
            source_code: data.program_source,
            state_frames: states_shifted,
        })
    }
    /// Peek at state with target number (0-based).
    /// 
    /// This will return None if the state doesn't exist.
    pub fn peek_at_state(&self, frame: usize) -> Option<&DebuggingState> {
        self.state_frames.get(frame)
    }

    /// Returns number of states. Counting starts from one.
    pub fn count_of_states(&self) -> usize {
        self.state_frames.len()
    }
}

/// One state of sed program execution.
///
/// Remembers state of sed program execution.
#[derive(Debug)]
pub struct DebuggingState<'a> {
    /// State of primary, or pattern, buffer
    pub pattern_buffer: String,
    /// State of secondary, or hold, buffer
    pub hold_buffer: String,
    /// If any regex was matched within the last execution step, the capture groups
    /// wil be saved here. If the previously executed instruction was not a substitution,
    /// this will be empty.
    pub matched_regex_registers: Vec<String>,
    /// Output of sed command. Each vec item means one line.
    pub output: Option<&'a str>,
    /// References current instruction in source code. This is computed heuristically
    /// and is not retrieved from inner sed state. So this might in some cases be wrong.
    /// If that's the case, file a bug.
    pub current_line: usize,
    /// Command executed by sed. With a bit of luck, this should match command referenced
    /// by current_line. If these two don't match, this one (`sed_command`) is right and
    /// a bug in parsing code occured.
    pub sed_command: Option<String>,
}
