use ratatui::{DefaultTerminal, Frame};
use sha1::{Digest, Sha1};
use std::{env, fs};

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

#[derive(Debug, Default)]
struct App {
    path: String,
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()?{
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            _ => {}
        }
    }

        fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_counter(&mut self) {
    }

    fn decrement_counter(&mut self) {
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.path.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}

struct Commit {
    hash: String,
    blob_path: String,
    description: String,
}

struct Tree {}

fn to_blob(file_path: &str) -> String {
    let contents = fs::read(file_path)
        .expect("Error reading file");

    let mut hasher = Sha1::new();
    hasher.update(&contents);

    let hash = hasher.finalize();
    let hash_string = hash.iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();

    hash_string
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let (_query, file_path) = parse_config(&args);

    let hash = to_blob(file_path);

    println!("SHA-1: {}", hash);

    // Ahora puedes comparar el hash con otro String.
    let other_hash = String::from("00e9c04bd4c8534bea1a2f26a7882765cd06fd7e");

    if hash == other_hash {
        println!("Los hashes coinciden");
    }


    // Run TUI
    ratatui::run(|terminal| App::default().run(terminal));
    Ok(())
}

fn parse_config(args: &[String]) -> (&str, &str) {
    let query = &args[1];
    let file_path = &args[2];

    (query, file_path)
}

fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(render)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}