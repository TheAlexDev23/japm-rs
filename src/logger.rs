use colored::Colorize;
use log::{Level, Log};

/// logger that logs to stdout/stderr
pub struct StdLogger {
    pub line_start: String,
}

impl Default for StdLogger {
    fn default() -> Self {
        StdLogger {
            line_start: String::from("==>"),
        }
    }
}

impl Log for StdLogger {
    fn log(&self, record: &log::Record) {
        let msg = format!("{}", record.args());
        let msg = msg.replace('\n', &format!("\n{} ", self.line_start));

        let message = format!("{} [{}] {}", self.line_start, record.level(), msg);
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
