use crate::sed::debugger::{Debugger, DebuggingState};
use crate::ui::generic::UiAgent;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
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
        Ok(Tui {
            debugger,
            terminal,
            forced_refresh_rate: 20,
        })
    }

    /// Generate layout and call individual draw methods for each layout part.
    fn draw<B: Backend>(
        f: &mut Frame<B>,
        debugger: &Debugger,
        state: &DebuggingState,
        breakpoints: &Vec<usize>,
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
        breakpoints: &Vec<usize>,
        focused_line: usize,
        area: Rect,
    ) {
        let block_source_code = Block::default()
            .title(" Source code ")
            .borders(Borders::ALL);
        let text = source_code.iter().map(|line| Text::raw(line));
        let mut text_output: Vec<Text> = Vec::new();
        // TODO: Implement scrolling
        for number in 0..source_code.len() {
            // TODO: Proper padding
            let linenr_color = if breakpoints.contains(&(number + 1)) {
                Color::LightRed
            } else {
                Color::Yellow
            };
            let linenr_bg_color = if number == focused_line {
                Color::DarkGray
            } else {
                Color::Reset
            };
            text_output.push(Text::styled(
                format!("{: <4}", (number + 1)),
                Style::default().fg(linenr_color).bg(linenr_bg_color),
            ));
            text_output.push(Text::raw(source_code.get(number).unwrap()));
            text_output.push(Text::raw("\n"));
        }
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
        let currentState = self.debugger._mock_state().unwrap();
        let mut breakpoints: Vec<usize> = Vec::new();

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
                        tx.send(Interrupt::UserEvent(key));
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
            let debugger = &self.debugger;
            let line_number = currentState.current_line;
            // Wait for interrupt
            match rx.recv().unwrap() {
                // Handle user input. Vi-like controls are available,
                // including prefixing a command with number to execute it
                // multiple times (in case of breakpoint toggles breakpoint on given line).
                // TODO: Add vi-like number command prefixing
                Interrupt::UserEvent(event) => match event.code {
                    // Exit
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    // Move cursor down
                    KeyCode::Char('j') | KeyCode::Down => {}
                    // Move cursor up
                    KeyCode::Char('k') | KeyCode::Up => {}
                    // Go to top of file
                    KeyCode::Char('g') => {}
                    // Go to bottom of file
                    KeyCode::Char('G') => {}
                    // Toggle breakpoint on current line
                    KeyCode::Char('b') => {}
                    // Step forward
                    KeyCode::Char('s') => {}
                    // Step backwards
                    KeyCode::Char('a') => {}
                    // Run till end or breakpoint
                    KeyCode::Char('r') => {}
                    // TODO: handle other keycodes (particulary numbers)
                    // TODO: Handle <Ctrl-C> signal
                    other => {}
                },
                Interrupt::IntervalElapsed => {}
            }
            // Draw
            self.terminal.draw(|mut f| {
                Tui::draw(&mut f, debugger, &currentState, &breakpoints, line_number)
            });
        }
    }
}

/// Why did we wake up drawing thread?
enum Interrupt {
    UserEvent(KeyEvent),
    IntervalElapsed,
}
