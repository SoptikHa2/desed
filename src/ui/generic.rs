use crate::sed::debugger::Debugger;

/// This trait describes structure that takes care of
/// interacting with user.
pub trait UiAgent {
    /// Start the agent that will now take over.
    fn start(self) -> Result<(), String>;
}
