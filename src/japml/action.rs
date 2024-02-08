use super::package::Package;

#[derive(Clone, Debug)]
pub struct Action {
    pub action_type: ActionType,
    pub package: Package,
}

#[derive(Clone, Debug)]
pub enum ActionType {
    Install,
    Remove,
}

impl Action {
    pub fn commit(self) {
        todo!("Not implemented yet");
    }
}