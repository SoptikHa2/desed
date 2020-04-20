use crate::sed::debugger::Debugger;

/// This trait describes structure that takes care of
/// interacting with user.
trait UiAgent {
    /// Create new agent using a built debugger.
    fn new<T>(debugger: Debugger) -> Result<(), T>
    where
        T: UiAgent;
    /// Start the agent that will now take over.
    fn start(self) -> Result<(), String>;
}
