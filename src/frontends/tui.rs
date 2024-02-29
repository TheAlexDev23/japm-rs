use std::io;
use std::io::Stderr;

use clap::error::Result;
use tokio::select;

use thiserror::Error;

use ratatui::{
    prelude::{CrosstermBackend, Rect, Terminal},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::action::Action;
use crate::frontends::messaging::UIReadHandle;

use super::MessageColor;

struct TuiHandle<'a> {
    messaging_handle: UIReadHandle,

    messages_window: TextWindow<'a>,
    actions_window: TextWindow<'a>,
    progressbar_window: ProgressbarWindow,
    terminal: Terminal<CrosstermBackend<Stderr>>,
}

struct TextWindow<'a> {
    title: String,
    render_threshold: u16,
    buffer: Text<'a>,
    rect: Rect,
}

struct ProgressbarWindow {
    progress: f32,
    rect: Rect,
}

#[derive(Error, Debug)]
pub enum InitializeError {
    #[error("An IO error has occured: {0}")]
    IO(#[from] io::Error),
    #[error("Your terminal size {0}x{1} is less than the required {2}x{3}")]
    Size(u16, u16, u16, u16),
}

pub fn init(read_handle: UIReadHandle) -> Result<(), InitializeError> {
    let mut handle = TuiHandle::init(read_handle)?;
    tokio::spawn(async move { handle.update_cycle().await });

    Ok(())
}

impl<'a> TuiHandle<'a> {
    pub fn init(read_handle: UIReadHandle) -> Result<TuiHandle<'a>, InitializeError> {
        const PROGRESSBAR_HEIGHT: u16 = 1;
        const ACTIONS_WINDOW_SCALE: f32 = 0.2;

        let (width, height) = crossterm::terminal::size()?;
        Self::verify_size(width, height)?;

        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

        let actions_width = (width as f32 * ACTIONS_WINDOW_SCALE) as u16;
        let messages_width = width - actions_width;
        let messages_height = height - PROGRESSBAR_HEIGHT;

        let messages_rect = Rect {
            x: 0,
            y: 0,
            height: messages_height,
            width: messages_width,
        };

        let progressbar_rect = Rect {
            x: 0,
            y: messages_height,
            height: PROGRESSBAR_HEIGHT,
            width,
        };

        let actions_rect = Rect {
            x: messages_width,
            y: 0,
            height: messages_height,
            width: actions_width,
        };

        let message_render_threshold = messages_rect.height;

        let handle = TuiHandle::<'a> {
            messaging_handle: read_handle,
            messages_window: TextWindow {
                title: String::from("Output"),
                render_threshold: message_render_threshold,
                buffer: Text::default(),
                rect: messages_rect,
            },
            actions_window: TextWindow {
                title: String::from("Completed actions"),
                render_threshold: message_render_threshold,
                buffer: Text::default(),
                rect: actions_rect,
            },
            progressbar_window: ProgressbarWindow {
                progress: 0.0,
                rect: progressbar_rect,
            },
            terminal: Terminal::new(CrosstermBackend::new(std::io::stderr()))?,
        };

        Ok(handle)
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

    pub(self) async fn update_cycle(&mut self) {
        loop {
            if self.handle_input().await {
                return;
            }

            self.terminal
                .draw(|frame| {
                    self.messages_window.render(frame);
                    self.actions_window.render(frame);

                    frame.render_widget(
                        Gauge::default().percent((self.progressbar_window.progress * 100.0) as u16),
                        self.progressbar_window.rect,
                    )
                })
                .expect("Could not draw terminal");
        }
    }

    async fn handle_input(&mut self) -> bool {
        select! {
            Some((message, color)) = self.messaging_handle.messages.recv() => {
                let style = match color {
                    MessageColor::White => Style::default().white(),
                    MessageColor::Cyan => Style::default().cyan(),
                    MessageColor::Green => Style::default().green(),
                    MessageColor::Yellow => Style::default().yellow(),
                    MessageColor::Purple => Style::default().magenta(),
                };

                self.messages_window
                    .buffer
                    .lines
                    .push(Line::styled(message, style));

                false
            }
            Some(action) = self.messaging_handle.actions.recv() => {
                let style = match action {
                    Action::Remove(_) => Style::default().red(),
                    Action::Install(_) => Style::default().green(),
                };

                self.actions_window
                    .buffer
                    .lines
                    .push(Line::styled(format!("{action}"), style));

                false
            }
            Some(percentage) = self.messaging_handle.progressbar.recv() => {
                self.progressbar_window.progress = percentage;

                false
            }
            Some(_) = self.messaging_handle.exit.recv() => {
                crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)
                    .expect("Could not leave alternate screen");
                crossterm::terminal::disable_raw_mode().expect("Could not disable raw mode");

                self.messaging_handle.exit_finish.lock().await.send(()).expect("Could not send exit finish response to frontend caller");

                true
            }
        }
    }
}

impl<'a> TextWindow<'a> {
    fn render(&self, frame: &mut Frame) {
        let mut scroll = self.buffer.lines.len() as i32 - self.render_threshold as i32;

        if scroll < 0 {
            scroll = 0;
        }

        frame.render_widget(
            Paragraph::new(self.buffer.clone())
                .scroll((scroll as u16, 0))
                .block(
                    Block::default()
                        .title(self.title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                ),
            self.rect,
        );
    }
}
