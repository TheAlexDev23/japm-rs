use core::fmt;
use std::{fmt::Display, fs::File};

use colored::Colorize;

pub struct Logger {
    log_curses: bool,
    line_start: String,
    error_log_file: Vec<File>,
    log_file: Vec<File>,
}

pub enum LogLevel {
    Dev,
    Inf,
    Err,
    Crit,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Dev => write!(f, "Development"),
            LogLevel::Inf=> write!(f, "Information"),
            LogLevel::Err => write!(f, "Error"),
            LogLevel::Crit => write!(f, "Critical"),
        }
    }
}

impl Logger {
    pub fn new(curses: bool) -> Logger {
        Logger {
            log_curses: curses,
            line_start: String::from("===>"),
            error_log_file: Vec::new(),
            log_file: Vec::new(),
        }
    }

    pub fn log(&self, msg: String, level: LogLevel) {
        if self.log_curses {
            todo!("Curses not yet supported");
        } else {
            let message = format!("{} {} {}", self.line_start, level, msg);
            match level {
                LogLevel::Dev => println!("{}", message.yellow()),
                LogLevel::Inf=> println!("{}", message.white()),
                LogLevel::Err => eprintln!("{}", message.red()),
                LogLevel::Crit => eprintln!("{}", message.purple()),
            }
        }
    }

    pub fn inf(&self, msg: String) {
        self.log(msg, LogLevel::Inf)
    }

    pub fn err(&self, msg: String) {
        self.log(msg, LogLevel::Err)
    }

    pub fn crit(&self, msg: String) {
        self.log(msg, LogLevel::Crit)
    }
}