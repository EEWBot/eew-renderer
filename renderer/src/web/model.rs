use crate::model::Message;

#[derive(Debug)]
pub(in crate::web) struct Shutdowner {
    producer: tokio::sync::mpsc::Sender<Message>,
}

impl Shutdowner {
    pub(in crate::web) fn new(producer: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self { producer }
    }
}

impl Drop for Shutdowner {
    fn drop(&mut self) {
        self.producer.blocking_send(Message::Shutdown).unwrap();
    }
}
