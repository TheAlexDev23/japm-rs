use std::sync::{Mutex, MutexGuard};

use crate::frontends;

#[derive(Debug)]
pub enum ProgressType {
    Setup,
    Packages,
    ActionsBuild,
    ActionsCommit,
}
pub trait Progress {
    fn increment_target(&mut self, progress_type: ProgressType, amount: i32);
    fn increment_completed(&mut self, progress_type: ProgressType, amount: i32);

    fn set_comleted(&mut self, progress_type: ProgressType);
}

pub struct FrontendProgress {
    setup: ProgressGroup,
    packages: ProgressGroup,
    actions_build: ProgressGroup,
    actions_commit: ProgressGroup,
}

struct ProgressGroup {
    pub completed: i32,
    pub target: i32,
}

impl FrontendProgress {
    pub fn new() -> Self {
        FrontendProgress {
            setup: ProgressGroup::new(),
            packages: ProgressGroup::new(),
            actions_build: ProgressGroup::new(),
            actions_commit: ProgressGroup::new(),
        }
    }

    pub fn refresh(&self) {
        let multiplier = 1.0 / 4.0;
        let progress: f32 = self.setup.get_progress() * multiplier
            + self.packages.get_progress() * multiplier
            + self.actions_build.get_progress() * multiplier
            + self.actions_commit.get_progress() * multiplier;

        frontends::set_progressbar(progress);
    }

    fn progress_group(&mut self, progress_type: ProgressType) -> &mut ProgressGroup {
        match progress_type {
            ProgressType::Setup => &mut self.setup,
            ProgressType::Packages => &mut self.packages,
            ProgressType::ActionsBuild => &mut self.actions_build,
            ProgressType::ActionsCommit => &mut self.actions_commit,
        }
    }
}

impl Progress for FrontendProgress {
    fn increment_target(&mut self, progress_type: ProgressType, amount: i32) {
        self.progress_group(progress_type).target += amount;
        self.refresh();
    }
    fn increment_completed(&mut self, progress_type: ProgressType, amount: i32) {
        self.progress_group(progress_type).completed += amount;
        self.refresh();
    }
    fn set_comleted(&mut self, progress_type: ProgressType) {
        let progress_group = self.progress_group(progress_type);
        progress_group.completed = 1;
        progress_group.target = 1;
        self.refresh();
    }
}

impl ProgressGroup {
    pub fn new() -> ProgressGroup {
        ProgressGroup {
            completed: 0,
            target: 0,
        }
    }

    pub fn get_progress(&self) -> f32 {
        if self.target == 0 {
            0.0
        } else {
            self.completed as f32 / self.target as f32
        }
    }
}

static mut CURRENT_PROGRESS: Option<Mutex<Box<dyn Progress>>> = None;

pub fn set_boxed_progress(progress: Box<dyn Progress>) {
    unsafe {
        CURRENT_PROGRESS = Some(Mutex::new(progress));
    }
}

pub fn increment_target(progress_type: ProgressType, amount: i32) {
    get_progress().increment_target(progress_type, amount)
}
pub fn increment_completed(progress_type: ProgressType, amount: i32) {
    get_progress().increment_completed(progress_type, amount)
}
pub fn set_comleted(progress_type: ProgressType) {
    get_progress().set_comleted(progress_type)
}

fn get_progress<'a>() -> MutexGuard<'a, Box<dyn Progress>> {
    unsafe {
        #[allow(clippy::mut_mutex_lock)]
        CURRENT_PROGRESS
            .as_mut()
            .unwrap()
            .lock()
            .expect("Could not lock progress")
    }
}
