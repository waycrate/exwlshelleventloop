use futures::{
    channel::mpsc,
    task::{Context, Poll},
    Sink,
};
use std::pin::Pin;

/// An event loop proxy that implements `Sink`.
/// NOTE: not proxy anything now
#[derive(Debug, Clone, Copy)]
pub struct IcedProxy;

impl<Message: 'static> Sink<Message> for IcedProxy {
    type Error = mpsc::SendError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, _message: Message) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
