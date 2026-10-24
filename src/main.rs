use std::{env, fs, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Wrap},
};
use sha1::{Digest, Sha1};

/// Hash to compare against (replace with the one you need).
const EXPECTED_HASH: &str = "00e9c04bd4c8534bea1a2f26a7882765cd06fd7e";

#[derive(Debug, Default)]
struct App {
    /// Text typed by the user (file path).
    input: String,
    /// Result of the last hash computation: Ok(hash) or Err(message).
    result: Option<Result<String, String>>,
    exit: bool,
}

impl App {
    fn new(initial_path: Option<String>) -> Self {
        let mut app = App::default();
        if let Some(path) = initial_path {
            app.input = path;
            app.compute_hash();
        }
        app
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let [input_area, result_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(5)]).areas(frame.area());

        // --- Input box ---
        let input_block = Block::bordered()
            .title(Line::from(" File path ".bold()).centered())
            .border_set(border::THICK);
        frame.render_widget(
            Paragraph::new(self.input.as_str().yellow()).block(input_block),
            input_area,
        );

        // Place the terminal cursor at the end of the typed text.
        let max_x = input_area.right().saturating_sub(2);
        let cursor_x = (input_area.x + 1 + self.input.chars().count() as u16).min(max_x);
        frame.set_cursor_position((cursor_x, input_area.y + 1));

        // --- Result box ---
        let instructions = Line::from(vec![
            " Compute ".into(),
            "<Enter>".blue().bold(),
            " Clear ".into(),
            "<Ctrl+U>".blue().bold(),
            " Quit ".into(),
            "<Esc> ".blue().bold(),
        ]);
        let result_block = Block::bordered()
            .title(Line::from(" SHA-1 ".bold()).centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let text = match &self.result {
            None => Text::from("Type a file path and press Enter.".dim()),
            Some(Ok(hash)) => {
                let verdict = if hash == EXPECTED_HASH {
                    "Matches the expected hash".green().bold()
                } else {
                    "Does not match the expected hash".red().bold()
                };
                Text::from(vec![
                    Line::from(vec!["SHA-1: ".into(), hash.clone().green()]),
                    Line::from(""),
                    Line::from(verdict),
                ])
            }
            Some(Err(message)) => Text::from(Line::from(message.clone().red())),
        };

        frame.render_widget(
            Paragraph::new(text)
                .centered()
                .wrap(Wrap { trim: false })
                .block(result_block),
            result_area,
        );
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Event::Key(key_event) = event::read()? {
            if key_event.kind == KeyEventKind::Press {
                self.handle_key_event(key_event);
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
        match key_event.code {
            KeyCode::Esc => self.exit = true,
            KeyCode::Char('c') if ctrl => self.exit = true,
            KeyCode::Char('u') if ctrl => {
                self.input.clear();
                self.result = None;
            }
            KeyCode::Enter => self.compute_hash(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) if !ctrl => self.input.push(c),
            _ => {}
        }
    }

    fn compute_hash(&mut self) {
        // Terminals often paste paths wrapped in quotes; strip them.
        let path = self.input.trim().trim_matches(|c| c == '"' || c == '\'');

        if path.is_empty() {
            self.result = Some(Err("Please enter a file path.".to_string()));
            return;
        }

        self.result = Some(to_blob(path).map_err(|e| format!("Could not read '{path}': {e}")));
    }
}

struct Commit {
    hash: String,
    blob_path: String,
    description: String,
}

struct Tree {}

fn to_blob(file_path: &str) -> io::Result<String> {
    let contents = fs::read(file_path)?;

    let mut hasher = Sha1::new();
    hasher.update(&contents);

    let hash = hasher.finalize();
    Ok(hash.iter().map(|byte| format!("{:02x}", byte)).collect())
}

fn main() -> io::Result<()> {
    // Optional: a path passed on the command line pre-fills the input.
    let initial_path = env::args().nth(1);

    ratatui::run(|terminal| App::new(initial_path).run(terminal))
}
