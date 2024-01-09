use crate::model::UserEvent;

#[derive(Debug)]
pub(in crate::endpoint) struct Shutdowner {
    producer: winit::event_loop::EventLoopProxy<UserEvent>,
}

impl Shutdowner {
    pub(in crate::endpoint) fn new(producer: winit::event_loop::EventLoopProxy<UserEvent>) -> Self {
        Self { producer }
    }
}

impl Drop for Shutdowner {
    fn drop(&mut self) {
        self.producer.send_event(UserEvent::Shutdown).unwrap();
    }
}

