use futures::{
    Sink,
    channel::mpsc,
    task::{Context, Poll},
};
use std::pin::Pin;

use std::sync::mpsc as stdmpsc;

/// An event loop proxy that implements `Sink`.
/// NOTE: not proxy anything now
#[derive(Debug)]
pub struct IcedProxy<Message: 'static>(stdmpsc::Sender<Message>);

impl<Message: 'static> Clone for IcedProxy<Message> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<Message: 'static> IcedProxy<Message> {
    pub fn new(sender: stdmpsc::Sender<Message>) -> Self {
        Self(sender)
    }
    #[allow(unused)]
    pub fn send_action(&self, action: Message) {
        self.0.send(action).ok();
    }
}

impl<Message: 'static> Sink<Message> for IcedProxy<Message> {
    type Error = mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, message: Message) -> Result<(), Self::Error> {
        self.0.send(message).ok();
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
