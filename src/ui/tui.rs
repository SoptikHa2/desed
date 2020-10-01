use crate::file_watcher::FileWatcher;
use crate::sed::debugger::{Debugger, DebuggingState};
use crate::ui::generic::{ApplicationExitReason, UiAgent};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, MouseEvent};
use crossterm::execute;
use std::cmp::{max, min};
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::terminal::Frame;
use tui::widgets::{Block, Borders, Paragraph, Text};
use tui::Terminal;

pub struct Tui<'a> {
    debugger: &'a Debugger,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    file_watcher: FileWatcher,
    /// Collection of lines which are designated as breakpoints
    breakpoints: HashSet<usize>,
    /// Remembers which line has user selected (has cursor on).
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
    /// Remembers at which state are we currently. User can step back and forth.
    current_state: usize,
}
impl<'a> Tui<'a> {
    /// Create new TUI that gathers data from the debugger.
    ///
    /// This consumes the debugger, as it's used to advance debugging state.
    #[allow(unused_must_use)]
    // NOTE: We don't care that some actions here fail (for example mouse handling),
    // as some features that we're trying to enable here are not necessary for desed.
    pub fn new(debugger: &'a Debugger, file_watcher: FileWatcher, current_state: usize) -> Result<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, event::EnableMouseCapture);
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)
            .with_context(|| "Failed to initialize terminal with crossterm backend.")?;
        crossterm::terminal::enable_raw_mode()?;
        terminal.hide_cursor();
        Ok(Tui {
            debugger,
            terminal,
            file_watcher,
            breakpoints: HashSet::new(),
            cursor: 0,
            forced_refresh_rate: 200,
            pressed_keys_buffer: String::new(),
            current_state
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
    fn draw_layout_and_subcomponents<B: Backend>(
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
            // Define colors depending whether currently selected line has a breakpoint
            let linenr_color = if breakpoints.contains(&line_number) {
                Color::LightRed
            } else {
                Color::Yellow
            };
            // Define background color depending on whether we have cursor here
            let linenr_bg_color = if line_number == cursor {
                Color::DarkGray
            } else {
                Color::Reset
            };
            // Format line indicator. It's different if the currently executing line is here
            let linenr_format = if line_number == interpreter_line {
                format!("{: <3}▶", (line_number + 1))
            } else {
                format!("{: <4}", (line_number + 1))
            };
            // Send the line we defined earlier to be displayed
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
                    format!("\n\\{}    ", i),
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

    /// Use crossterm and stdout to restore terminal state.
    ///
    /// This shall be called on application exit.
    #[allow(unused_must_use)]
    // NOTE: We don't care if we fail to do something here. Terminal might not support everything,
    // but we try to restore as much as we can.
    pub fn restore_terminal_state() -> Result<()> {
        let mut stdout = io::stdout();
        // Disable mouse control
        execute!(stdout, event::DisableMouseCapture);
        // Disable raw mode that messes up with user's terminal and show cursor again
        crossterm::terminal::disable_raw_mode();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.show_cursor();
        // And clear as much as we can before handing the control of terminal back to user.
        terminal.clear();
        Ok(())
    }
}

impl<'a> UiAgent for Tui<'a> {
    fn start(mut self) -> Result<ApplicationExitReason> {
        // Setup event loop and input handling
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(self.forced_refresh_rate);
        let mut file_watcher = self.file_watcher;

        // Thread that will send interrupt singals to UI thread (this one)
        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                // Oh we got an event from user
                // UNWRAP: We need to use it because I don't know how to return Result
                // from this, and I doubt it can even be done.
                if event::poll(tick_rate - last_tick.elapsed()).unwrap() {
                    // Send interrupt
                    // UNWRAP: We are guaranteed that the following call will succeed
                    // as we know there already something waiting for us (see event::poll)
                    let event = event::read().unwrap();
                    if let Event::Key(key) = event {
                        if let Err(_) = tx.send(Interrupt::KeyPressed(key)) {
                            return;
                        }
                    } else if let Event::Mouse(mouse) = event {
                        if let Err(_) = tx.send(Interrupt::MouseEvent(mouse)) {
                            return;
                        }
                    }
                }
                if file_watcher.any_events().ok().unwrap_or(false) {
                    if let Err(_) = tx.send(Interrupt::FileChanged) {
                        return;
                    }
                }
                if last_tick.elapsed() > tick_rate {
                    if let Err(_) = tx.send(Interrupt::IntervalElapsed) {
                        return;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        self.terminal.clear().with_context(|| {
            "Failed to clear terminal during drawing state. Do you have modern term?"
        })?;
        let mut use_execution_pointer_as_focus_line = false;
        let mut draw_memory: DrawMemory = DrawMemory::default();

        // UI thread that manages drawing
        loop {
            let current_state = self.debugger.peek_at_state(self.current_state)
                .with_context(||"We got ourselves into impossible state. This is logical error, please report a bug.")?;
            let debugger = &self.debugger;
            let line_number = current_state.current_line;
            // Wait for interrupt
            match rx.recv()? {
                // Handle user input. Vi-like controls are available,
                // including prefixing a command with number to execute it
                // multiple times (in case of breakpoint toggles breakpoint on given line).
                Interrupt::KeyPressed(event) => match event.code {
                    // Exit
                    KeyCode::Char('q') => {
                        return Ok(ApplicationExitReason::UserExit);
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
                            if self.current_state < debugger.count_of_states() - 1 {
                                self.current_state += 1;
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
                            if self.current_state > 0 {
                                self.current_state -= 1;
                            }
                        }
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
                    }
                    // Run till end or breakpoint
                    KeyCode::Char('r') => {
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
                        while self.current_state < debugger.count_of_states() - 1 {
                            self.current_state += 1;
                            if self.breakpoints.contains(&self.debugger.peek_at_state(self.current_state).unwrap().current_line) {
                                break;
                            }
                        }
                    },
                    // Same as 'r', but backwards
                    KeyCode::Char('R') => {
                        use_execution_pointer_as_focus_line = true;
                        self.pressed_keys_buffer.clear();
                        while self.current_state > 0 {
                            self.current_state -= 1;
                            if self.breakpoints.contains(&self.debugger.peek_at_state(self.current_state).unwrap().current_line) {
                                break;
                            }
                        }
                    },
                    // Reload source code and try to enter current state again
                    KeyCode::Char('l') => {
                        return Ok(ApplicationExitReason::Reload(self.current_state));
                    }
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
                Interrupt::FileChanged => {
                    return Ok(ApplicationExitReason::Reload(self.current_state));
                }
                Interrupt::IntervalElapsed => {}
            }
            // Draw
            let breakpoints = &self.breakpoints;
            let cursor = self.cursor;
            self.terminal.draw(|mut f| {
                Tui::draw_layout_and_subcomponents(
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
            })?
        }
    }
}

/// Why did we wake up drawing thread?
enum Interrupt {
    KeyPressed(KeyEvent),
    MouseEvent(MouseEvent),
    FileChanged,
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
