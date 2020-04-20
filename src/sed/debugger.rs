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
    pub soure_code: Vec<String>,
    /// Source code that was highlighted with ANSI codes, one instruction per line.
    ///
    /// If there were multiple instructions on a single line in original source code,
    /// they are spread out so one is on each line.
    pub highlighted_source_code: Vec<String>,
    /// Previously visited debugging states, inclding the current one.
    ///
    /// See `history_limit` for maximum debugging states stored.
    /// We rotate them afterwards.
    state_frames: VecDeque<DebuggingState>,
    /// Maximum limit of debugging states saved.
    ///
    /// As we want the option to traverse program execution state
    /// back and forth, we save the debugging states to memory.
    ///
    /// However, with extremely (and likely maliciously crafted - when
    /// did you last needed *that* large sed program) large sed programs,
    /// this might occupy too much memory - especially since we actually
    /// store new copy of the pattern and hold buffer each step.
    ///
    /// If this is set, there will be a limit to maximum of state frames that
    /// will be saved into memory.
    ///
    /// This will never be zero. If it is, debugger will panic.
    history_limit: Option<usize>,
}
impl Debugger {
    pub fn new() -> Self {
        unimplemented!();
    }
    pub fn next_state<'a>() -> &'a DebuggingState {
        unimplemented!();
    }
    pub fn previous_state<'a>() -> &'a DebuggingState {
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
