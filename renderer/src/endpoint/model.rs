use crate::model::UserEvent;

#[derive(Debug)]
pub(in crate::endpoint) struct Shutdowner {
    producer: tokio::sync::mpsc::Sender<UserEvent>,
}

impl Shutdowner {
    pub(in crate::endpoint) fn new(producer: tokio::sync::mpsc::Sender<UserEvent>) -> Self {
        Self { producer }
    }
}

impl Drop for Shutdowner {
    fn drop(&mut self) {
        self.producer.blocking_send(UserEvent::Shutdown).unwrap();
    }
}
