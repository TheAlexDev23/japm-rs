use std::io;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use super::{Frontend, MessageColor};

pub struct StdFrontend {
    terminal_width: u16,
    progressbar: ProgressBar,
}

impl StdFrontend {
    pub fn new() -> Result<StdFrontend, io::Error> {
        let (width, _) = crossterm::terminal::size()?;
        let progressbar = ProgressBar::new(width as u64);
        progressbar.set_style(
            ProgressStyle::with_template("{wide_bar}")
                .unwrap()
                .progress_chars("██ "),
        );

        Ok(StdFrontend {
            terminal_width: width,
            progressbar,
        })
    }
}

impl Frontend for StdFrontend {
    fn refresh(&mut self) {}

    fn display_message(&mut self, message: String, color: &super::MessageColor) {
        match color {
            MessageColor::White => self.progressbar.println(format!("{}", message.white())),
            MessageColor::Cyan => self.progressbar.println(format!("{}", message.cyan())),
            MessageColor::Green => self.progressbar.println(format!("{}", message.green())),
            MessageColor::Yellow => self.progressbar.println(format!("{}", message.yellow())),
            MessageColor::Purple => self.progressbar.println(format!("{}", message.purple())),
        }
    }

    fn display_action(&mut self, _: &crate::action::Action) {}

    fn set_progressbar(&mut self, percentage: f32) {
        self.progressbar
            .set_position((self.terminal_width as f32 * percentage) as u64)
    }

    fn exit(&mut self) {
        self.progressbar.finish_and_clear();
    }
}
