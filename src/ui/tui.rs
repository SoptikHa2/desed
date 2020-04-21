use crate::sed::debugger::{Debugger, DebuggingState};
use crate::ui::generic::UiAgent;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::collections::HashSet;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::terminal::Frame;
use tui::widgets::{Block, Borders, Paragraph, Text};
use tui::Terminal;

pub struct Tui {
    debugger: Debugger,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    breakpoints: HashSet<usize>,
    cursor: usize,
    /// UI is refreshed automatically on user input.
    /// However if no user input arrives, how often should
    /// application redraw itself anyway?
    ///
    /// This is in milliseconds. For example value of 100
    /// means that the application is refreshed at least once every
    /// 100 milliseconds.
    forced_refresh_rate: u64,
}
impl Tui {
    pub fn new(debugger: Debugger) -> Result<Self, String> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        crossterm::terminal::enable_raw_mode();
        terminal.hide_cursor();
        Ok(Tui {
            debugger,
            terminal,
            breakpoints: HashSet::new(),
            cursor: 0,
            forced_refresh_rate: 200,
        })
    }

    /// Generate layout and call individual draw methods for each layout part.
    fn draw<B: Backend>(
        f: &mut Frame<B>,
        debugger: &Debugger,
        state: &DebuggingState,
        breakpoints: &HashSet<usize>,
        // Line (0-based) which user has selected via cursor
        cursor: usize,
        // Line (0-based) which sed interpreter currently executes
        interpreter_line: usize,
        // Line (0-based) which should be approximately at the center of the screen
        focused_line: usize,
    ) {
        let total_size = f.size();

        if let [left_plane, right_plane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
            .split(total_size)[0..2]
        {
            if let [pattern_plane, hold_plane, regex_match_plane] = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(1, 3),
                    ]
                    .as_ref(),
                )
                .split(right_plane)[0..3]
            {
                Tui::draw_source_code(
                    f,
                    &debugger.source_code,
                    breakpoints,
                    focused_line,
                    cursor,
                    interpreter_line,
                    left_plane,
                );
                Tui::draw_text(
                    f,
                    String::from(" Pattern space "),
                    &state.pattern_buffer,
                    pattern_plane,
                );
                Tui::draw_text(
                    f,
                    String::from(" Hold space "),
                    &state.hold_buffer,
                    hold_plane,
                );
                Tui::draw_regex_space(f, &state.matched_regex_registers, regex_match_plane);
            } else {
                panic!("Failed to generate vertically split layout 1:1:1.");
            }
        } else {
            panic!("Failed to generate horizontally split layout 2:3.");
        }
    }

    /// Draw source code into main window.
    ///
    /// Handles scrolling and breakpoint display as well.
    ///
    /// TODO: syntax highlighting
    fn draw_source_code<B: Backend>(
        f: &mut Frame<B>,
        source_code: &Vec<String>,
        breakpoints: &HashSet<usize>,
        focused_line: usize,
        cursor: usize,
        interpreter_line: usize,
        area: Rect,
    ) {
        let block_source_code = Block::default()
            .title(" Source code ")
            .borders(Borders::ALL);
        let text = source_code.iter().map(|line| Text::raw(line));
        let mut text_output: Vec<Text> = Vec::new();
        // TODO: Implement scrolling
        // Define closure that prints one more line of source code
        let mut add_new_line = |line_number| {
            let linenr_color = if breakpoints.contains(&line_number) {
                Color::LightRed
            } else {
                Color::Yellow
            };
            let linenr_bg_color = if line_number == cursor {
                Color::DarkGray
            } else {
                Color::Reset
            };
            let linenr_format = if line_number == interpreter_line {
                format!("{: <3}▶", (line_number + 1))
            } else {
                format!("{: <4}", (line_number + 1))
            };
            text_output.push(Text::styled(
                linenr_format,
                Style::default().fg(linenr_color).bg(linenr_bg_color),
            ));
            if let Some(source) = source_code.get(line_number) {
                text_output.push(Text::raw(source));
            }
            text_output.push(Text::raw("\n"));
        };
        for number in 0..source_code.len() {
            add_new_line(number);
        }
        // TODO: Do this only when we scrolled down and see end of file
        // Add one more "phantom" line so we see line where current segment execution ends
        add_new_line(source_code.len());
        let paragraph = Paragraph::new(text_output.iter())
            .block(block_source_code)
            .wrap(true);
        f.render_widget(paragraph, area);
    }

    /// Draw regex. This either prints "No matches" in dark gray, italics if there are no matches,
    /// or prints all the matches with their capture group number beforehand.
    fn draw_regex_space<B: Backend>(f: &mut Frame<B>, regex_space: &Vec<String>, area: Rect) {
        let block_regex_space = Block::default()
            .title(" Regex matches ")
            .borders(Borders::ALL);
        let mut text: Vec<Text> = Vec::new();
        if regex_space.len() == 0 {
            text.push(Text::styled(
                "\nNo matches",
                Style::default()
                    .modifier(Modifier::ITALIC)
                    .fg(Color::DarkGray),
            ));
        } else {
            for (i, m) in regex_space.iter().enumerate() {
                text.push(Text::styled(
                    format!("\n\\{}    ", (i + 1)),
                    Style::default().fg(Color::DarkGray),
                ));
                text.push(Text::raw(m));
            }
        }
        let paragraph = Paragraph::new(text.iter())
            .block(block_regex_space)
            .wrap(true);
        f.render_widget(paragraph, area);
    }

    /// Draw simple text in area, wrapping, with light blue fg color. Do nothing else.
    fn draw_text<B: Backend>(
        f: &mut Frame<B>,
        heading: String,
        text_to_write: &String,
        area: Rect,
    ) {
        let block = Block::default().title(&heading).borders(Borders::ALL);
        let text = [Text::styled(
            format!("\n{}", text_to_write),
            Style::default().fg(Color::LightBlue),
        )];
        let paragraph = Paragraph::new(text.iter()).block(block).wrap(true);
        f.render_widget(paragraph, area);
    }
}

impl UiAgent for Tui {
    fn start(mut self) -> std::result::Result<(), std::string::String> {
        let mut current_state = self.debugger.current_state().ok_or(String::from(
            "It looks like the source code loaded was empty. Nothing to do.",
        ))?;

        // Setup event loop and input handling
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(self.forced_refresh_rate);

        // Thread that will send interrupt singals to UI thread (this one)
        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                // Oh we got an event from user
                if event::poll(tick_rate - last_tick.elapsed()).unwrap() {
                    // Send interrupt
                    if let Event::Key(key) = event::read().unwrap() {
                        tx.send(Interrupt::UserEvent(key)).unwrap();
                    }
                }
                if last_tick.elapsed() > tick_rate {
                    tx.send(Interrupt::IntervalElapsed).unwrap();
                    last_tick = Instant::now();
                }
            }
        });

        self.terminal.clear();

        // UI thread that manages drawing
        loop {
            let debugger = &mut self.debugger;
            let line_number = current_state.current_line;
            // Wait for interrupt
            match rx.recv().unwrap() {
                // Handle user input. Vi-like controls are available,
                // including prefixing a command with number to execute it
                // multiple times (in case of breakpoint toggles breakpoint on given line).
                // TODO: Add vi-like number command prefixing
                Interrupt::UserEvent(event) => match event.code {
                    // Exit
                    KeyCode::Char('q') => {
                        // Terminal might go crazy if we don't switch the mode back
                        crossterm::terminal::disable_raw_mode();
                        return Ok(());
                    }
                    // Move cursor down
                    KeyCode::Char('j') | KeyCode::Down => {
                        if self.cursor < debugger.source_code.len() {
                            self.cursor += 1;
                        }
                    }
                    // Move cursor up
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                        }
                    }
                    // Go to top of file
                    KeyCode::Char('g') => {
                        self.cursor = 0;
                    }
                    // Go to bottom of file
                    KeyCode::Char('G') => {
                        self.cursor = debugger.source_code.len();
                    }
                    // Toggle breakpoint on current line
                    KeyCode::Char('b') => {
                        if self.breakpoints.contains(&self.cursor) {
                            self.breakpoints.remove(&self.cursor);
                        } else {
                            self.breakpoints.insert(self.cursor);
                        }
                    }
                    // Step forward
                    KeyCode::Char('s') => {
                        if let Some(newstate) = debugger.next_state() {
                            current_state = newstate;
                        }
                    }
                    // Step backwards
                    KeyCode::Char('a') => {
                        if let Some(prevstate) = debugger.previous_state() {
                            current_state = prevstate;
                        }
                    }
                    // Run till end or breakpoint
                    KeyCode::Char('r') => loop {
                        if let Some(newstate) = debugger.next_state() {
                            current_state = newstate;
                            if self.breakpoints.contains(&current_state.current_line) {
                                break;
                            }
                        } else {
                            break;
                        }
                    },
                    // TODO: handle other keycodes (particulary numbers)
                    other => {}
                },
                Interrupt::IntervalElapsed => {}
            }
            // Draw
            let bp = &self.breakpoints;
            let c = self.cursor;
            self.terminal
                .draw(|mut f| Tui::draw(&mut f, debugger, &current_state, &bp, c, line_number, c))
                .unwrap();
        }
    }
}

/// Why did we wake up drawing thread?
enum Interrupt {
    UserEvent(KeyEvent),
    IntervalElapsed,
}
