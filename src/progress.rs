use tokio::sync::{Mutex, MutexGuard};

use crate::frontends;

#[derive(Debug)]
pub enum ProgressType {
    Setup,
    Packages,
    ActionsBuild,
    ActionsCommit,
}

#[async_trait::async_trait]
pub trait Progress {
    async fn increment_target(&mut self, progress_type: ProgressType, amount: i32);
    async fn increment_completed(&mut self, progress_type: ProgressType, amount: i32);

    async fn set_comleted(&mut self, progress_type: ProgressType);
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

    pub async fn refresh(&self) {
        let multiplier = 1.0 / 4.0;
        let progress: f32 = self.setup.get_progress() * multiplier
            + self.packages.get_progress() * multiplier
            + self.actions_build.get_progress() * multiplier
            + self.actions_commit.get_progress() * multiplier;

        frontends::set_progressbar(progress).await;
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

#[async_trait::async_trait]
impl Progress for FrontendProgress {
    async fn increment_target(&mut self, progress_type: ProgressType, amount: i32) {
        self.progress_group(progress_type).target += amount;
        self.refresh().await;
    }
    async fn increment_completed(&mut self, progress_type: ProgressType, amount: i32) {
        self.progress_group(progress_type).completed += amount;
        self.refresh().await;
    }
    async fn set_comleted(&mut self, progress_type: ProgressType) {
        let progress_group = self.progress_group(progress_type);
        progress_group.completed = 1;
        progress_group.target = 1;
        self.refresh().await;
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

pub async fn increment_target(progress_type: ProgressType, amount: i32) {
    get_progress()
        .await
        .increment_target(progress_type, amount)
        .await;
}
pub async fn increment_completed(progress_type: ProgressType, amount: i32) {
    get_progress()
        .await
        .increment_completed(progress_type, amount)
        .await;
}
pub async fn set_comleted(progress_type: ProgressType) {
    get_progress().await.set_comleted(progress_type).await;
}

async fn get_progress<'a>() -> MutexGuard<'a, Box<dyn Progress>> {
    unsafe {
        #[allow(clippy::mut_mutex_lock)]
        CURRENT_PROGRESS.as_mut().unwrap().lock().await
    }
}
