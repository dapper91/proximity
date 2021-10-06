use tokio::sync::watch;

#[derive(Copy, Clone)]
pub enum Signal {
    Init,
    Stop,
    Reload,
}

pub struct Sender {
    sender: watch::Sender<Signal>,
}

impl Sender {
    pub fn send(&self, sig: Signal) -> Option<()> {
        self.sender.send(sig).ok()
    }
}

#[derive(Clone)]
pub struct Receiver {
    receiver: watch::Receiver<Signal>,
}

impl Receiver {
    pub async fn receive(&mut self) -> Signal {
        self.receiver.changed().await;
        return *self.receiver.borrow();
    }
}

pub fn signaler() -> (Sender, Receiver) {
    let (sender, receiver) = watch::channel(Signal::Init);

    return (Sender { sender }, Receiver { receiver });
}
