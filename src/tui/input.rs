/// Result of handling a key press in the TUI event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
    Continue,
    Quit,
}
