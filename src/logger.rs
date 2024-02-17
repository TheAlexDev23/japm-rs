use colored::Colorize;
use log::{Level, Log};
use ratatui::style::{Style, Stylize};
use std::sync::{Arc, Mutex};

use crate::tui::TuiManager;

const LINE_START: &str = "==>";

/// logger that logs to stdout/stderr
pub struct StdLogger;

pub struct TuiLogger<'a> {
    tui: Arc<Mutex<TuiManager<'a>>>,
}

impl Default for StdLogger {
    fn default() -> Self {
        StdLogger
    }
}
impl<'a> TuiLogger<'a> {
    pub fn new(tui: Arc<Mutex<TuiManager<'a>>>) -> Self {
        TuiLogger { tui }
    }
}

impl Log for StdLogger {
    fn log(&self, record: &log::Record) {
        let msg = format!("{}", record.args());

        let message =
            colored::ColoredString::from(format!("{} [{}] {}", LINE_START, record.level(), msg));

        match record.level() {
            Level::Trace => println!("{}", message.white()),
            Level::Debug => println!("{}", message.cyan()),
            Level::Info => println!("{}", message.green()),
            Level::Warn => eprintln!("{}", message.yellow()),
            Level::Error => eprintln!("{}", message.purple()),
        }
    }

    fn enabled(&self, _: &log::Metadata) -> bool {
        // Idk how this is supposed to work if we set the logging level when setting the global
        // logger. And this function afaik doesn't even get called
        true
    }

    fn flush(&self) {}
}

impl<'a> Log for TuiLogger<'a> {
    fn log(&self, record: &log::Record) {
        let mut tui = self.tui.lock().unwrap();

        let msg = format!("{}", record.args());
        let message = format!("{} [{}] {}", LINE_START, record.level(), msg);

        for line in message.split('\n').map(String::from) {
            match record.level() {
                Level::Trace => tui.print_message(line, Style::default().white()),
                Level::Debug => tui.print_message(line, Style::default().cyan()),
                Level::Info => tui.print_message(line, Style::default().green()),
                Level::Warn => tui.print_message(line, Style::default().yellow()),
                Level::Error => tui.print_message(line, Style::default().magenta()),
            }
            .expect("Could not log message");
        }
    }

    fn enabled(&self, _: &log::Metadata) -> bool {
        // Idk how this is supposed to work if we set the logging level when setting the global
        // logger. And this function afaik doesn't even get called
        true
    }

    fn flush(&self) {}
}
