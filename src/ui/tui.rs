use crate::sed::debugger::{Debugger, DebuggingState};
use crate::ui::generic::UiAgent;
use std::io;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, Widget};
use tui::Terminal;

pub struct Tui {
    debugger: Debugger,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}
impl Tui {
    pub fn new(debugger: Debugger) -> std::result::Result<Self, String> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).unwrap();
        Ok(Tui { debugger, terminal })
    }
}
impl UiAgent for Tui {
    fn start(mut self) -> std::result::Result<(), std::string::String> {
        //let currentState = self.debugger.next_state();
        self.terminal.draw(|mut f| {
            let size = f.size();
            let block = Block::default().title("Block").borders(Borders::ALL);
            f.render_widget(block, size);
        });

        Ok(())
    }
}
