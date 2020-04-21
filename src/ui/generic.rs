/// This trait describes structure that takes care of
/// interacting with user.
pub trait UiAgent {
    /// Start the agent that will now take over.
    fn start(self) -> Result<ApplicationExitReason, String>;
}

/// Used to indicate why did UiAgent stop
pub enum ApplicationExitReason {
    /// User wants to exit the application
    UserExit,
    /// User wants to reload configuration.
    ///
    /// usize: state ID that should be loaded again if possible
    Reload(usize),
}
