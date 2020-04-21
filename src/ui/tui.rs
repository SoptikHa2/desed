use crate::sed::debugger::{Debugger, DebuggingState};
use crate::ui::generic::UiAgent;
use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent};
use crossterm::QueueableCommand;
use std::cmp::{max, min};
use std::collections::HashSet;
use std::io::{self};
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
    /// Pressed keys that should be stored but can't be processed now.
    /// For example, user can press "10k". The "1" and "0" are important and should
    /// be stored, but can't be executed because we don't know what to do (move up) until
    /// we see the "k" character. The instant we see it, we read the whole buffer and clear it.
    pressed_keys_buffer: String,
}
impl Tui {
    pub fn new(debugger: Debugger) -> Result<Self, String> {
        let mut stdout = io::stdout();
        stdout.queue(event::EnableMouseCapture);
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
            pressed_keys_buffer: String::new(),
        })
    }

    /// Reads given buffer and returns it as a number.
    ///
    /// A default value will be return if the number is non-parsable (typically empty buffer) or is
    /// not at least 1.
    fn get_pressed_key_buffer_as_number(buffer: &String, default_value: usize) -> usize {
        if let Ok(num) = buffer.parse() {
            if num >= 1 {
                num
            } else {
                default_value
            }
        } else {
            default_value
        }
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
        draw_memory: &mut DrawMemory,
    ) {
        let total_size = f.size();

        if let [left_plane, right_plane] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)].as_ref())
            .split(total_size)[..]
        {
            if let [pattern_plane, hold_plane, regex_match_plane, output_plane] = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(1, 4),
                    ]
                    .as_ref(),
                )
                .split(right_plane)[..]
            {
                Tui::draw_source_code(
                    f,
                    &debugger.source_code,
                    breakpoints,
                    focused_line,
                    cursor,
                    interpreter_line,
                    draw_memory,
                    left_plane,
                );
                Tui::draw_text(
                    f,
                    String::from(" Pattern space "),
                    Some(&state.pattern_buffer),
                    pattern_plane,
                );
                Tui::draw_text(
                    f,
                    String::from(" Hold space "),
                    Some(&state.hold_buffer),
                    hold_plane,
                );
                Tui::draw_regex_space(f, &state.matched_regex_registers, regex_match_plane);
                Tui::draw_text(
                    f,
                    String::from(" Output "),
                    state.output.as_ref().map(|s| s.join("\n")).as_ref(),
                    output_plane,
                );
            } else {
                panic!("Failed to generate vertically split layout 1:1:1:1.");
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
        draw_memory: &mut DrawMemory,
        area: Rect,
    ) {
        let block_source_code = Block::default()
            .title(" Source code ")
            .borders(Borders::ALL);
        let mut text_output: Vec<Text> = Vec::new();

        // Scroll:
        // Focused line is line that should always be at the center of the screen.
        let display_start;
        {
            let grace_lines = 10;
            let height = area.height as i32;
            let previous_startline = draw_memory.current_startline;
            // Minimum startline that should be possible to have in any case
            let minimum_startline = 0;
            // Maximum startline that should be possible to have in any case
            // Magical number 4: I don't know what it's doing here, but it works this way. Otherwise
            // we just keep maximum scroll four lines early.
            let maximum_startline = (source_code.len() as i32 - 1) - height + 4;
            // Minimum startline position that makes sense - we want visible code but within limits of the source code height.
            let mut minimum_viable_startline = max(
                focused_line as i32 - height + grace_lines,
                minimum_startline,
            ) as usize;
            // Maximum startline position that makes sense - we want visible code but within limits of the source code height
            let mut maximum_viable_startline = max(
                min(focused_line as i32 - grace_lines, maximum_startline),
                minimum_startline,
            ) as usize;
            // Sometimes, towards end of file, maximum and minim viable lines have swapped values.
            // No idea why, but swapping them helps the problem.
            if minimum_viable_startline > maximum_viable_startline {
                minimum_viable_startline ^= maximum_viable_startline;
                maximum_viable_startline ^= minimum_viable_startline;
                minimum_viable_startline ^= maximum_viable_startline;
            }
            // Try to keep previous startline as it was, but scroll up or down as
            // little as possible to keep within bonds
            if previous_startline < minimum_viable_startline {
                display_start = minimum_viable_startline;
            } else if previous_startline > maximum_viable_startline {
                display_start = maximum_viable_startline;
            } else {
                display_start = previous_startline;
            }
            draw_memory.current_startline = display_start;
        }

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
        for number in display_start..source_code.len() {
            add_new_line(number);
        }
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
        text_to_write: Option<&String>,
        area: Rect,
    ) {
        let block = Block::default().title(&heading).borders(Borders::ALL);
        let default_string = String::new();
        let text = [Text::styled(
            format!("\n{}", text_to_write.unwrap_or(&default_string)),
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
                    let event = event::read().unwrap();
                    if let Event::Key(key) = event {
                        tx.send(Interrupt::KeyPressed(key)).unwrap();
                    } else if let Event::Mouse(mouse) = event {
                        tx.send(Interrupt::MouseEvent(mouse)).unwrap();
                    }
                }
                if last_tick.elapsed() > tick_rate {
                    tx.send(Interrupt::IntervalElapsed).unwrap();
                    last_tick = Instant::now();
                }
            }
        });

        self.terminal.clear();
        let mut use_execution_pointer_as_focus_line = false;
        let mut draw_memory: DrawMemory = DrawMemory::default();

        // UI thread that manages drawing
        loop {
            let debugger = &mut self.debugger;
            let line_number = current_state.current_line;
            // Wait for interrupt
            match rx.recv().unwrap() {
                // Handle user input. Vi-like controls are available,
                // including prefixing a command with number to execute it
                // multiple times (in case of breakpoint toggles breakpoint on given line).
                Interrupt::KeyPressed(event) => match event.code {
                    // Exit
                    KeyCode::Char('q') => {
                        // Terminal might go crazy if we don't switch the mode back
                        crossterm::terminal::disable_raw_mode();
                        // Disable our weird mouse handling so we don't break mouse handling of parent terminal
                        let mut stdout = io::stdout();
                        stdout.queue(event::DisableMouseCapture);
                        return Ok(());
                    }
                    // Move cursor down
                    KeyCode::Char('j') | KeyCode::Down => {
                        for _ in
                            0..Tui::get_pressed_key_buffer_as_number(&self.pressed_keys_buffer, 1)
                        {
                            if self.cursor < debugger.source_code.len() {
                                self.cursor += 1;
                            }
                        }
                        use_execution_pointer_as_focus_line = false;
                        self.pressed_keys_buffer.clear();
                    }
                    // Move cursor up
                    KeyCode::Char('k') | KeyCode::Up => {
                        for _ in
                            0..Tui::get_pressed_key_buffer_as_number(&self.pressed_keys_buffer, 1)
                        {
                            if self.cursor > 0 {
                                self.cursor -= 1;
                            }
                        }
                        use_execution_pointer_as_focus_line = false;
                        self.pressed_keys_buffer.clear();
                    }
                    // Go to top of file
                    KeyCode::Char('g') => {
                        self.cursor = 0;
                        use_execution_pointer_as_focus_line = false;
                        self.pressed_keys_buffer.clear();
                    }
                    // Go to bottom of file
                    KeyCode::Char('G') => {
                        self.cursor = debugger.source_code.len();
                        use_execution_pointer_as_focus_line = false;
                        self.pressed_keys_buffer.clear();
                    }
                    // Toggle breakpoint on current line
                    KeyCode::Char('b') => {
                        let mut breakpoint_target =
                            Tui::get_pressed_key_buffer_as_number(&self.pressed_keys_buffer, 0);
                        if breakpoint_target == 0 {
                            breakpoint_target = self.cursor;
                        } else {
                            breakpoint_target -= 1;
                        }
                        if self.breakpoints.contains(&breakpoint_target) {
                            self.breakpoints.remove(&breakpoint_target);
                        } else {
                            self.breakpoints.insert(breakpoint_target);
                        }
                        self.pressed_keys_buffer.clear();
                    }
                    // Step forward
                    KeyCode::Char('s') => {
                        for _ in
                            0..Tui::get_pressed_key_buffer_as_number(&self.pressed_keys_buffer, 1)
                        {
                            if let Some(newstate) = debugger.next_state() {
                                current_state = newstate;
                            }
                        }
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
                    }
                    // Step backwards
                    KeyCode::Char('a') => {
                        for _ in
                            0..Tui::get_pressed_key_buffer_as_number(&self.pressed_keys_buffer, 1)
                        {
                            if let Some(prevstate) = debugger.previous_state() {
                                current_state = prevstate;
                            }
                        }
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
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
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
                    },
                    KeyCode::Char('R') => loop {
                        if let Some(newstate) = debugger.previous_state() {
                            current_state = newstate;
                            if self.breakpoints.contains(&current_state.current_line) {
                                break;
                            }
                        } else {
                            break;
                        }
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
                    },
                    KeyCode::Char(other) => match other {
                        '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                            self.pressed_keys_buffer.push(other);
                        }
                        _ => {
                            // Invalid key, clear buffer
                            self.pressed_keys_buffer.clear();
                        }
                    },
                    _ => {
                        self.pressed_keys_buffer.clear();
                    }
                },
                Interrupt::MouseEvent(event) => match event {
                    // Button pressed, mark current line as breakpoint
                    MouseEvent::Up(_button, _col, row, _key_modifiers) => {
                        let target_breakpoint = (row - 1) as usize + draw_memory.current_startline;
                        if self.breakpoints.contains(&target_breakpoint) {
                            self.breakpoints.remove(&target_breakpoint);
                        } else {
                            self.breakpoints.insert(target_breakpoint);
                        }
                    }
                    MouseEvent::ScrollUp(_col, _row, _key_modifiers) => {
                        if self.cursor > 0 {
                            self.cursor -= 1;
                        }
                        use_execution_pointer_as_focus_line = false;
                    }
                    MouseEvent::ScrollDown(_col, _row, _key_modifiers) => {
                        if self.cursor < debugger.source_code.len() {
                            self.cursor += 1;
                        }
                        use_execution_pointer_as_focus_line = false;
                    }
                    _ => {}
                },
                Interrupt::IntervalElapsed => {}
            }
            // Draw
            let breakpoints = &self.breakpoints;
            let cursor = self.cursor;
            self.terminal
                .draw(|mut f| {
                    Tui::draw(
                        &mut f,
                        debugger,
                        &current_state,
                        &breakpoints,
                        cursor,
                        line_number,
                        if use_execution_pointer_as_focus_line {
                            line_number
                        } else {
                            cursor
                        },
                        &mut draw_memory,
                    )
                })
                .unwrap();
        }
    }
}

/// Why did we wake up drawing thread?
enum Interrupt {
    KeyPressed(KeyEvent),
    MouseEvent(MouseEvent),
    IntervalElapsed,
}

/// This is currently used to remember last scroll
/// position so screen doesn't wiggle as much.
struct DrawMemory {
    current_startline: usize,
}
impl DrawMemory {
    fn default() -> Self {
        DrawMemory {
            current_startline: 0,
        }
    }
}
