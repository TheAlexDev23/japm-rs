use std::io;

use tokio::select;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::frontends::messaging::UIReadHandle;

use super::MessageColor;

struct StdHandle {
    messaging_handle: UIReadHandle,

    terminal_width: u16,
    progressbar: ProgressBar,
}

pub fn init(read_handle: UIReadHandle) -> Result<(), io::Error> {
    let mut handle = StdHandle::init(read_handle)?;
    tokio::spawn(async move { handle.update_cycle().await });

    Ok(())
}

impl StdHandle {
    pub fn init(read_handle: UIReadHandle) -> Result<StdHandle, io::Error> {
        let (width, _) = crossterm::terminal::size()?;
        let progressbar = ProgressBar::new(width as u64);
        progressbar.set_style(
            ProgressStyle::with_template("{wide_bar}")
                .unwrap()
                .progress_chars("██ "),
        );

        Ok(StdHandle {
            messaging_handle: read_handle,
            terminal_width: width,
            progressbar,
        })
    }

    pub(self) async fn update_cycle(&mut self) {
        loop {
            if self.handle_input().await {
                return;
            }
        }
    }

    async fn handle_input(&mut self) -> bool {
        select! {
            Some((message, color)) = self.messaging_handle.messages.recv() => {
                match color {
                    MessageColor::White => self.progressbar.println(format!("{}", message.white())),
                    MessageColor::Cyan => self.progressbar.println(format!("{}", message.cyan())),
                    MessageColor::Green => self.progressbar.println(format!("{}", message.green())),
                    MessageColor::Yellow => self.progressbar.println(format!("{}", message.yellow())),
                    MessageColor::Purple => self.progressbar.println(format!("{}", message.purple())),
                }

                false
            }
            Some(percentage) = self.messaging_handle.progressbar.recv() => {
                self.progressbar
                    .set_position((self.terminal_width as f32 * percentage) as u64);

                false
            }
            Some(_) = self.messaging_handle.exit.recv() => {
                self.progressbar.finish_and_clear();

                true
            }
        }
    }
}
