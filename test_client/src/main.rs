use nix::sys::socket::{SockAddr, VsockAddr};
use tokio_vsock::VsockStream;
use libc;
use futures::{Ready, Async};

use tokio::prelude::*;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = VsockStream::connect(&SockAddr::Vsock(VsockAddr::new(
        3,
        8000
    ))).await?;
    
    stream.write(b"hello world!").await?;
    
    // let mut buf = Vec::new();
    // println!("{:?}", stream.read(&mut buf).await?);
    
    match stream.poll_read_ready(Ready::readable()) {
        Ok(Async::Ready(_)) => println!("read ready"),
        Ok(Async::NotReady) => println!("not read ready"),
        Err(e) => eprintln!("got error: {}", e),
    };

    Ok(())
}
