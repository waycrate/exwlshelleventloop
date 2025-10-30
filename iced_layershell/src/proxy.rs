use futures::{
    Sink,
    channel::mpsc,
    task::{Context, Poll},
};
use iced_graphics::shell;
use iced_runtime::{Action, window};
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

impl<T: 'static> IcedProxy<T> {
    pub fn new(sender: stdmpsc::Sender<T>) -> Self {
        Self(sender)
    }
    #[allow(unused)]
    pub fn send_action(&self, action: T) {
        self.0.send(action).ok();
    }
}

impl<Message: 'static> Sink<Action<Message>> for IcedProxy<Action<Message>> {
    type Error = mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, message: Action<Message>) -> Result<(), Self::Error> {
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

impl<T> shell::Notifier for IcedProxy<Action<T>>
where
    T: Send,
{
    fn request_redraw(&self) {
        self.send_action(Action::Window(window::Action::RedrawAll));
    }

    fn invalidate_layout(&self) {
        self.send_action(Action::Window(window::Action::RelayoutAll));
    }
}
