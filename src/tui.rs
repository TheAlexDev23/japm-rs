use std::io;
use std::io::Stderr;

use thiserror::Error;

use ratatui::{
    prelude::{CrosstermBackend, Rect, Terminal},
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
};

pub struct TuiManager<'a> {
    messages_window: MessagesWindow<'a>,
    progressbar_window: ProgressbarWindow,
    terminal: Terminal<CrosstermBackend<Stderr>>,
}

struct MessagesWindow<'a> {
    render_threshold: u16,
    buffer: Text<'a>,
    rect: Rect,
}

struct ProgressbarWindow {
    rect: Rect,
}

#[derive(Error, Debug)]
pub enum InitializeError {
    #[error("An IO error has occured: {0}")]
    IO(#[from] io::Error),
    #[error("Your terminal size {0}x{1} is less than the required {2}x{3}")]
    Size(u16, u16, u16, u16),
}

impl<'a> TuiManager<'a> {
    pub fn initialize() -> Result<TuiManager<'a>, InitializeError> {
        const PROGRESSBAR_HEIGHT: u16 = 1;

        let (width, height) = crossterm::terminal::size()?;
        Self::verify_size(width, height)?;

        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

        let messages_rect = Rect {
            x: 0,
            y: 0,
            height: height - PROGRESSBAR_HEIGHT,
            width,
        };

        let progressbar_rect = Rect {
            x: 0,
            y: height - PROGRESSBAR_HEIGHT,
            height: PROGRESSBAR_HEIGHT,
            width,
        };

        let message_render_threshold = messages_rect.height;

        Ok(TuiManager {
            messages_window: MessagesWindow {
                render_threshold: message_render_threshold,
                buffer: Text::default(),
                rect: messages_rect,
            },
            progressbar_window: ProgressbarWindow {
                rect: progressbar_rect,
            },
            terminal: Terminal::new(CrosstermBackend::new(std::io::stderr()))?,
        })
    }

    fn verify_size(width: u16, height: u16) -> Result<(), InitializeError> {
        const MIN_HEIGHT: u16 = 40;
        const MIN_WIDTH: u16 = 100;

        if height < MIN_HEIGHT || width < MIN_WIDTH {
            Err(InitializeError::Size(width, height, MIN_WIDTH, MIN_HEIGHT))
        } else {
            Ok(())
        }
    }

    pub fn print_message(&mut self, message: String, style: Style) -> Result<(), io::Error> {
        self.messages_window
            .buffer
            .lines
            .push(Line::styled(message, style));

        self.refresh()
    }

    pub fn refresh(&mut self) -> Result<(), io::Error> {
        let mut scroll = self.messages_window.buffer.lines.len() as i32
            - self.messages_window.render_threshold as i32;

        if scroll < 0 {
            scroll = 0;
        }

        self.terminal.draw(|frame| {
            frame.render_widget(
                Paragraph::new(self.messages_window.buffer.clone())
                    .scroll((scroll as u16, 0))
                    .block(
                        Block::default()
                            .title("Ouptut")
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    ),
                self.messages_window.rect,
            );
            frame.render_widget(Gauge::default().percent(20), self.progressbar_window.rect)
        })?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), io::Error> {
        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;

        Ok(())
    }
}
