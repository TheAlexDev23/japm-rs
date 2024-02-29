use std::sync::Arc;

use crate::action::Action;

use messaging::UIWriteHandle;

pub mod messaging;
pub mod stdout;
pub mod tui;

#[derive(Clone)]
pub enum MessageColor {
    White,
    Cyan,
    Green,
    Yellow,
    Purple,
}

static mut UI_MESSENGER: Option<Arc<UIWriteHandle>> = None;

pub fn set_ui_messenger(messenger: UIWriteHandle) {
    unsafe {
        UI_MESSENGER = Some(Arc::new(messenger));
    }
}

pub async fn display_message(message: String, color: &MessageColor) -> Option<()> {
    get_messenger()?.display_message(message, color).await;
    Some(())
}
pub async fn display_action(action: &Action) -> Option<()> {
    get_messenger()?.display_action(action).await;
    Some(())
}
pub async fn set_progressbar(percentage: f32) -> Option<()> {
    get_messenger()?.set_progressbar(percentage).await;
    Some(())
}
pub async fn exit() -> Option<()> {
    let messenger = get_messenger()?;
    messenger.exit().await;
    messenger.exit_finish.lock().await.recv().await;
    Some(())
}

fn get_messenger<'a>() -> Option<&'a UIWriteHandle> {
    unsafe { UI_MESSENGER.as_deref() }
}
