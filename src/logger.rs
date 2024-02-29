use std::io;

use log::{Level, Log};

use crate::frontends::{self, MessageColor};

const LINE_START: &str = "==>";

pub struct FrontendLogger {
    rt: tokio::runtime::Runtime,
}

impl FrontendLogger {
    pub fn new() -> Result<FrontendLogger, io::Error> {
        Ok(FrontendLogger {
            rt: tokio::runtime::Runtime::new()?,
        })
    }
}

impl Log for FrontendLogger {
    fn log(&self, record: &log::Record) {
        let msg = format!("{}", record.args());
        let message = format!("{} [{}] {}", LINE_START, record.level(), msg);

        let color = match record.level() {
            Level::Trace => MessageColor::White,
            Level::Debug => MessageColor::Cyan,
            Level::Info => MessageColor::Green,
            Level::Warn => MessageColor::Yellow,
            Level::Error => MessageColor::Purple,
        };

        self.rt.spawn(async move {
            let mut message = message.split('\n').map(String::from);
            frontends::display_message(message.next().unwrap(), &color).await;

            for mut line in message {
                line.insert_str(0, "    ");
                frontends::display_message(line, &color).await;
            }
        });
    }

    fn enabled(&self, _: &log::Metadata) -> bool {
        // Idk how this is supposed to work if we set the logging level when setting the global
        // logger. And this function afaik doesn't even get called
        true
    }

    fn flush(&self) {}
}
