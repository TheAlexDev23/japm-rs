use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;

use crate::Action;

use super::MessageColor;

pub struct UIWriteHandle {
    messages: Mutex<UnboundedSender<(String, MessageColor)>>,
    actions: Mutex<UnboundedSender<Action>>,
    progressbar: Mutex<UnboundedSender<f32>>,
    exit: Mutex<UnboundedSender<()>>,

    /// The frontend will need to send to this receiver through [UIReadHandle::exit_finish]
    /// once the [Self::exit] procedure is finished.
    pub exit_finish: Mutex<UnboundedReceiver<()>>,
}

pub struct UIReadHandle {
    pub messages: UnboundedReceiver<(String, MessageColor)>,
    pub actions: UnboundedReceiver<Action>,
    pub progressbar: UnboundedReceiver<f32>,
    pub exit: UnboundedReceiver<()>,
    pub exit_finish: Mutex<UnboundedSender<()>>,
}

pub fn generate_message_pair() -> (UIWriteHandle, UIReadHandle) {
    let (mw, mr) = mpsc::unbounded_channel();
    let (aw, ar) = mpsc::unbounded_channel();
    let (pw, pr) = mpsc::unbounded_channel();
    let (ew, er) = mpsc::unbounded_channel();
    let (efw, efr) = mpsc::unbounded_channel();

    (
        UIWriteHandle {
            messages: mw.into(),
            actions: aw.into(),
            progressbar: pw.into(),
            exit: ew.into(),
            exit_finish: efr.into(),
        },
        UIReadHandle {
            messages: mr,
            actions: ar,
            progressbar: pr,
            exit: er,
            exit_finish: efw.into(),
        },
    )
}

impl UIWriteHandle {
    pub async fn display_message(&self, message: String, color: &MessageColor) {
        self.messages
            .lock()
            .await
            .send((message, color.clone()))
            .unwrap();
    }

    pub async fn display_action(&self, action: &Action) {
        self.actions.lock().await.send(action.clone()).unwrap();
    }

    pub async fn set_progressbar(&self, percentage: f32) {
        self.progressbar.lock().await.send(percentage).unwrap();
    }

    pub async fn exit(&self) {
        self.exit.lock().await.send(()).unwrap();
    }
}
