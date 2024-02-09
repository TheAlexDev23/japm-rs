use core::fmt;
use std::{fmt::Display, fs::File};

use colored::Colorize;

pub struct Logger {
    use_curses: bool,
    line_start: String,
    log_level: LogLevel,
    error_log_file: Vec<File>,
    log_file: Vec<File>,
}

#[derive(Clone, Copy)]
pub enum LogLevel {
    Dev,
    Inf,
    Err,
    Crit,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Dev => write!(f, "devel"),
            LogLevel::Inf => write!(f, "info"),
            LogLevel::Err => write!(f, "error"),
            LogLevel::Crit => write!(f, "critical"),
        }
    }
}

impl Logger {
    pub fn new(use_curses: bool, log_level: LogLevel) -> Logger {
        Logger {
            use_curses,
            line_start: String::from("===>"),
            log_level,
            error_log_file: Vec::new(),
            log_file: Vec::new(),
        }
    }

    pub fn log(&self, msg: impl Into<String>, level: LogLevel) {
        if (level as i32) < (self.log_level as i32) {
            return;
        }

        if self.use_curses {
            todo!("Curses not yet supported");
        } else {
            let msg = msg.into().replace('\n', &format!("\n{} ", self.line_start));

            let message = format!("{} [{}] {}", self.line_start, level, msg);
            match level {
                LogLevel::Dev => println!("{}", message.yellow()),
                LogLevel::Inf => println!("{}", message.green()),
                LogLevel::Err => eprintln!("{}", message.red()),
                LogLevel::Crit => eprintln!("{}", message.purple()),
            }
        }
    }

    pub fn dev(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Dev)
    }

    pub fn inf(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Inf)
    }

    pub fn err(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Err)
    }

    pub fn crit(&self, msg: impl Into<String>) {
        self.log(msg, LogLevel::Crit)
    }
}
