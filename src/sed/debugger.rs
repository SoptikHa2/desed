use crate::cli::Options;
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
    state_frames: Vec<DebuggingState>,
    current_frame: usize,
}
impl Debugger {
    /// Create new instance of debugger and launch sed.
    pub fn new(settings: Options) -> Result<Self, String> {
        let mut communicator = SedCommunicator::new(settings);
        let data = communicator.getExecutionInfoFromSed()?;
        Ok(Debugger {
            source_code: data.program_source,
            state_frames: data.states,
            current_frame: 0,
        })
    }
    /// Create new instance of debugger with mock data.
    /// Useful for UI testing.
    ///
    /// TODO: Provide more meaningful data here.
    pub fn _mock(settings: Options) -> Result<Self, String> {
        Ok(Debugger {
            source_code: vec!["source", "code", "example"]
                .iter()
                .map(|s| String::from(*s))
                .collect(),
            state_frames: Vec::new(),
            current_frame: 1,
        })
    }
    /// Create new instance of debugging state with mock data.
    /// Useful for UI testing.
    ///
    /// TODO: Provide more meaningful data here.
    pub fn _mock_state(&self) -> Option<DebuggingState> {
        Some(DebuggingState {
            pattern_buffer: String::from("helloworld"),
            hold_buffer: String::from(""),
            matched_regex_registers: vec!["hel", "orl"]
                .iter()
                .map(|s: &&str| String::from(*s))
                .collect(),
            current_line: 2,
            output: None,
            sed_command: Some(String::from("source")),
        })
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
        self.state_frames
            .get(self.current_frame - 1)
            .map(|s| s.clone())
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
        self.state_frames
            .get(self.current_frame + 1)
            .map(|s| s.clone())
    }
}

/// One state of sed program execution.
///
/// Remembers state of sed program execution.
#[derive(Clone)]
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
