use crate::progress::{Progress, ProgressType};

pub struct MockProgressbar;

#[async_trait::async_trait]
impl Progress for MockProgressbar {
    async fn increment_target(&mut self, _progress_type: ProgressType, _amount: i32) {}
    async fn increment_completed(&mut self, _progress_type: ProgressType, _amount: i32) {}
    async fn set_comleted(&mut self, _progress_type: ProgressType) {}
}
