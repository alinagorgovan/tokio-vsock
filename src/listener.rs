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

use std::io::{Error, ErrorKind, Result};
use std::os::unix::io::{AsRawFd, RawFd};

use futures::{try_ready, Async, Poll, Stream};
use nix::sys::socket::SockAddr;
use tokio::reactor::{Handle, PollEvented2};

use crate::stream::VsockStream;

/// An I/O object representing a Virtio socket listening for incoming connections.
#[derive(Debug)]
pub struct VsockListener {
    io: PollEvented2<super::mio::VsockListener>,
}

impl VsockListener {
    fn new(listener: super::mio::VsockListener) -> Self {
        let io = PollEvented2::new(listener);
        Self { io }
    }

    /// Create a new Virtio socket listener associated with this event loop.
    pub fn bind(addr: &SockAddr) -> Result<Self> {
        let l = super::mio::VsockListener::bind(addr)?;
        Ok(Self::new(l))
    }

    /// Attempt to accept a connection and create a new connected socket if
    /// successful.
    pub fn poll_accept(&mut self) -> Result<Async<(VsockStream, SockAddr)>> {
        let (io, addr) = try_ready!(self.poll_accept_std());

        let io = super::mio::VsockStream::from_std(io)?;
        let io = VsockStream::new(io);

        Ok((io, addr).into())
    }

    /// Attempt to accept a connection and create a new connected socket if
    /// successful.
    pub fn poll_accept_std(&mut self) -> Result<Async<(vsock::VsockStream, SockAddr)>> {
        try_ready!(self.io.poll_read_ready(mio::Ready::readable()));

        match self.io.get_ref().accept_std() {
            Ok(pair) => Ok(pair.into()),
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                self.io.clear_read_ready(mio::Ready::readable())?;
                Ok(Async::NotReady)
            }
            Err(e) => Err(e),
        }
    }

    /// Create a new Virtio socket listener from a blocking listener.
    pub fn from_std(listener: vsock::VsockListener, handle: &Handle) -> Result<Self> {
        let io = super::mio::VsockListener::from_std(listener)?;
        let io = PollEvented2::new_with_handle(io, handle)?;
        Ok(VsockListener { io })
    }

    /// The local address that this listener is bound to.
    pub fn local_addr(&self) -> Result<SockAddr> {
        self.io.get_ref().local_addr()
    }

    /// Consumes this listener, returning a stream of the sockets this listener
    /// accepts.
    pub fn incoming(self) -> Incoming {
        Incoming::new(self)
    }
}

impl AsRawFd for VsockListener {
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

/// Stream returned by the `VsockListener::incoming` representing sockets received from a listener.
#[derive(Debug)]
pub struct Incoming {
    inner: VsockListener,
}

impl Incoming {
    fn new(listener: VsockListener) -> Incoming {
        Incoming { inner: listener }
    }
}

impl Stream for Incoming {
    type Item = VsockStream;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Error> {
        let (socket, _) = try_ready!(self.inner.poll_accept());
        Ok(Async::Ready(Some(socket)))
    }
}
