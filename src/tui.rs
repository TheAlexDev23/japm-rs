use std::collections::VecDeque;
use std::io;
use std::io::Stderr;

use thiserror::Error;

use ratatui::{
    prelude::{CrosstermBackend, Rect, Terminal},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
};

pub struct TuiManager<'a> {
    messages_window: MessagesWindow<'a>,
    progressbar_window: ProgressbarWindow,
    terminal: Terminal<CrosstermBackend<Stderr>>,
}

struct MessagesWindow<'a> {
    render_threshold: u16,
    buffer: VecDeque<Line<'a>>,
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

        let message_render_threshold = messages_rect.height + 20;

        Ok(TuiManager {
            messages_window: MessagesWindow {
                render_threshold: message_render_threshold,
                buffer: VecDeque::with_capacity(message_render_threshold as usize),
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
        let message = Span::styled(message, style);

        self.messages_window.buffer.push_back(Line::from(message));
        if self.messages_window.buffer.len() == self.messages_window.render_threshold as usize {
            self.messages_window.buffer.pop_front();
        }

        self.refresh()
    }

    pub fn refresh(&mut self) -> Result<(), io::Error> {
        let text = Text::from(
            self.messages_window
                .buffer
                .iter()
                .cloned()
                .collect::<Vec<Line>>(),
        );

        self.terminal.draw(|frame| {
            frame.render_widget(
                Paragraph::new(text).block(
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
