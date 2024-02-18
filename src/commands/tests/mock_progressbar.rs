use super::{Progress, ProgressType};

pub struct MockProgressbar;

impl Progress for MockProgressbar {
    fn increment_target(&mut self, _progress_type: ProgressType, _amount: i32) {}
    fn increment_completed(&mut self, _progress_type: ProgressType, _amount: i32) {}
    fn set_comleted(&mut self, _progress_type: ProgressType) {}
}
