/*
 * Tokio Reference TCP Implementation
 * Copyright (c) 2019 Tokio Contributors
 *
 * Permission is hereby granted, free of charge, to any
 * person obtaining a copy of this software and associated
 * documentation files (the "Software"), to deal in the
 * Software without restriction, including without
 * limitation the rights to use, copy, modify, merge,
 * publish, distribute, sublicense, and/or sell copies of
 * the Software, and to permit persons to whom the Software
 * is furnished to do so, subject to the following
 * conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions
 * of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
 * ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
 * TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
 * PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
 * SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
 * CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
 * OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
 * IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */

/*
 * Copyright 2019 fsyncd, Berlin, Germany.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::io;
use std::os::unix::io::{AsRawFd, RawFd};

use mio::Ready;
use futures::ready;
use nix::sys::socket::SockAddr;
use futures::future::poll_fn;
use std::task::{Context, Poll};

use tokio::io::PollEvented;

use crate::stream::VsockStream;

/// An I/O object representing a Virtio socket listening for incoming connections.
#[derive(Debug)]
pub struct VsockListener {
    io: PollEvented<super::mio::VsockListener>,
}

impl VsockListener {
    /// Create a new Virtio socket listener associated with this event loop.
    pub fn bind(addr: &SockAddr) -> io::Result<VsockListener> {
        let listener = super::mio::VsockListener::bind(addr)?;
        let io = PollEvented::new(listener)?;
        Ok(VsockListener { io })
    }
    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SockAddr> {
        self.io.get_ref().local_addr()
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.io.get_ref().take_error()
    }

    /// Accepts a new incoming connection to this listener.
    pub async fn accept(&mut self) -> io::Result<(VsockStream, SockAddr)> {
        poll_fn(|cx| self.poll_accept(cx)).await
    }

    /// Attempt to accept a connection and create a new connected socket if
    /// successful.
    pub(crate) fn poll_accept(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<(VsockStream, SockAddr)>> {
        let (io, addr) = ready!(self.poll_accept_std(cx))?;

        let io = super::mio::VsockStream::from_std(io)?;
                
        Ok((VsockStream::new(io)?, addr)).into()
    }

    /// Attempt to accept a connection and create a new connected socket if
    /// successful.
    fn poll_accept_std(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<(vsock::VsockStream, SockAddr)>> {
        ready!(self.io.poll_read_ready(cx, Ready::readable()))?;

        match self.io.get_ref().accept_std() {
            Ok(None) => {
                self.io.clear_read_ready(cx, Ready::readable())?;
                Poll::Pending
            }
            Ok(Some((sock, addr))) => Ok((sock, addr)).into(),
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.io.clear_read_ready(cx, Ready::readable())?;
                Poll::Pending
            }
            Err(err) => Err(err).into(),
        }
    }

    /// Create a new Virtio socket listener from a blocking listener.
    pub fn from_std(listener: vsock::VsockListener) -> io::Result<Self> {
        let io = super::mio::VsockListener::from_std(listener)?;
        let io = PollEvented::new(io)?;
        Ok(VsockListener { io })
    }

    // Consumes this listener, returning a stream of the sockets this listener
    /// accepts.
    ///
    /// This method returns an implementation of the `Stream` trait which
    /// resolves to the sockets the are accepted on this listener.
    pub fn incoming(&mut self) -> super::Incoming<'_> {
        super::Incoming::new(self)
    }
}

impl AsRawFd for VsockListener {
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}
