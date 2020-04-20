use crate::cli::Options;
use crate::sed::communication::SedCommunicator;
use std::collections::VecDeque;

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
    state_frames: VecDeque<DebuggingState>,
}
impl Debugger {
    /// Create new instance of debugger and launch sed.
    pub fn new(settings: Options) -> Result<Self, String> {
        unimplemented!();
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
            state_frames: VecDeque::new(),
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
        })
    }
    /// Go to next sed execution step.
    ///
    /// This might return None if we reached end of execution.
    pub fn next_state(&self) -> Option<DebuggingState> {
        unimplemented!();
    }
    /// Go to previous sed execution step as saved in memory.
    ///
    /// This might return None if we are at start of execution or
    /// if there no longer any states left in history.
    pub fn previous_state(&self) -> Option<DebuggingState> {
        unimplemented!();
    }
}

/// One state of sed program execution.
///
/// Remembers state of sed program execution.
pub struct DebuggingState {
    /// State of primary, or pattern, buffer
    pub pattern_buffer: String,
    /// State of secondary, or hold, buffer
    pub hold_buffer: String,
    /// If any regex was matched within the last execution step, the capture groups
    /// wil be saved here. If the previously executed instruction was not a substitution,
    /// this will be empty.
    pub matched_regex_registers: Vec<String>,
    /// References current instruction in source code. This is computed heuristically
    /// and is not retrieved from inner sed state. So this might in some cases be wrong.
    /// If that's the case, file a bug.
    pub current_line: usize,
}
