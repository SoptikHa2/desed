use crate::cli::Options;
use crate::sed::communication::DebugInfoFromSed;
use crate::sed::communication::SedCommunicator;

/// Sed program debugger.
///
/// This stores debug states, allowing user
/// to return to previous states of execution.
///
/// This will panic if something bad happens
/// while executing sed, sed isn't the GNU version,
/// or an invalid inner state (which should never happen) happens.
pub struct Debugger {
    /// Sed source code, one instruction per line.
    ///
    /// If there were multiple instructions on a single line in original source code,
    /// they are spread out so one is on each line.
    pub source_code: Vec<String>,
    /// Previously visited debugging states, inclding the current one.
    ///
    /// See `history_limit` for maximum debugging states stored.
    /// We rotate them afterwards.
    pub state_frames: Vec<DebuggingState>,
    current_frame: usize,
}
impl Debugger {
    /// Create new instance of debugger and launch sed.
    pub fn new(settings: Options) -> Result<Self, String> {
        let communicator = SedCommunicator::new(settings);
        let data: DebugInfoFromSed = communicator.getExecutionInfoFromSed()?;
        // Shift all outputs and pattern matches one frame earlier.
        // The way it's done now (output appears one frame after it's source)
        // is, while the way sed works, very confusing.
        let mut states: Vec<DebuggingState> = data.states;
        states.reverse();
        let mut states_shifted: Vec<DebuggingState> = Vec::with_capacity(states.len());
        let mut previous_output: Option<Vec<String>> = None;
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
            current_frame: 0,
        })
    }
    pub fn current_state(&self) -> Option<DebuggingState> {
        // TODO: Solve this without cloning. This is awful.
        self.state_frames.get(self.current_frame).map(|s| s.clone())
    }
    /// Go to next sed execution step.
    ///
    /// This might return None if we reached end of execution.
    pub fn next_state(&mut self) -> Option<DebuggingState> {
        if self.current_frame >= self.state_frames.len() {
            return None;
        }
        self.current_frame += 1;
        // TODO: Solve this without cloning. This is awful.
        self.state_frames.get(self.current_frame).map(|s| s.clone())
    }
    /// Go to previous sed execution step as saved in memory.
    ///
    /// This might return None if we are at start of execution or
    /// if there no longer any states left in history.
    pub fn previous_state(&mut self) -> Option<DebuggingState> {
        if self.current_frame == 0 {
            return None;
        }
        self.current_frame -= 1;
        // TODO: Solve this without cloning. This is awful.
        self.state_frames.get(self.current_frame).map(|s| s.clone())
    }
}

/// One state of sed program execution.
///
/// Remembers state of sed program execution.
#[derive(Clone, Debug)]
pub struct DebuggingState {
    /// State of primary, or pattern, buffer
    pub pattern_buffer: String,
    /// State of secondary, or hold, buffer
    pub hold_buffer: String,
    /// If any regex was matched within the last execution step, the capture groups
    /// wil be saved here. If the previously executed instruction was not a substitution,
    /// this will be empty.
    pub matched_regex_registers: Vec<String>,
    /// Output of sed command. Each vec item means one line.
    pub output: Option<Vec<String>>,
    /// References current instruction in source code. This is computed heuristically
    /// and is not retrieved from inner sed state. So this might in some cases be wrong.
    /// If that's the case, file a bug.
    pub current_line: usize,
    /// Command executed by sed. With a bit of luck, this should match command referenced
    /// by current_line. If these two don't match, this one (`sed_command`) is right and
    /// a bug in parsing code occured.
    pub sed_command: Option<String>,
}
