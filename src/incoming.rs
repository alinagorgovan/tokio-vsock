use crate::{VsockListener, VsockStream};

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::stream::Stream;
use futures::ready;

/// Stream of listeners
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Incoming<'a> {
    inner: &'a mut VsockListener,
}

impl Incoming<'_> {
    pub(crate) fn new(listener: &mut VsockListener) -> Incoming<'_> {
        Incoming { inner: listener }
    }

    /// Attempts to poll `VsockStream` by polling inner `VsockListener` to accept
    /// connection.
    ///
    /// If `VsockListener` isn't ready yet, `Poll::Pending` is returned and
    /// current task will be notified by a waker.  Otherwise `Poll::Ready` with
    /// `Result` containing `VsockStream` will be returned.
    pub fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<VsockStream>> {
        let (socket, _) = ready!(self.inner.poll_accept(cx))?;
        Poll::Ready(Ok(socket))
    }
}

impl Stream for Incoming<'_> {
    type Item = io::Result<VsockStream>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let (socket, _) = ready!(self.inner.poll_accept(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}
