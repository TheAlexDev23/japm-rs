mod tui;

use std::sync::{Mutex, MutexGuard};

use crate::action::Action;

pub use tui::TuiFrontend;

pub enum MessageColor {
    White,
    Cyan,
    Green,
    Yellow,
    Purple,
}

static mut CURRENT_FRONTEND: Option<Mutex<Box<dyn Frontend>>> = None;

pub trait Frontend {
    fn refresh(&mut self);
    fn display_message(&mut self, message: String, color: &MessageColor);
    fn display_action(&mut self, action: &Action);
    fn set_progressbar(&mut self, percentage: f32);
    fn exit(&mut self);
}

pub fn set_boxed_frontend(frontend: Box<dyn Frontend>) {
    unsafe {
        CURRENT_FRONTEND = Some(Mutex::new(frontend));
    }
}

pub fn display_message(message: String, color: &MessageColor) {
    get_frontend().display_message(message, color);
}
pub fn display_action(action: &Action) {
    get_frontend().display_action(action);
}
pub fn set_progressbar(percentage: f32) {
    get_frontend().set_progressbar(percentage);
}
pub fn exit() {
    get_frontend().exit();
}

fn get_frontend<'a>() -> MutexGuard<'a, Box<dyn Frontend>> {
    unsafe {
        // Need to lock instead of get_mut because otherwise weird graphic fuckery happens that I don't really
        // understand
        #[allow(clippy::mut_mutex_lock)]
        CURRENT_FRONTEND
            .as_mut()
            .unwrap()
            .lock()
            .expect("Could not lock frontend instance.")
    }
}
