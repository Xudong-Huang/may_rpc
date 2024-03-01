//! general CS framework for rust based on coroutines with a focus on ease of use.
//!
//! the general communication procedure is as below
//! 1. client send request to server
//! 2. server recv request from client
//! 3. server parsing and process the request
//! 4. server send out response to client
//! 5. client recv response from server
//!
//! this lib provide a general request/response struct called `Frame`, it's just a wrapper for the raw
//! data `Vec<u8>`. you need to prepare and parsing it in the actual process functions that passed into
//! the framework
//!
pub use errors::{Error, WireError};
pub use frame::{Frame, ReqBuf, RspBuf};
pub use multiplex_client::MultiplexClient;
pub use server::{ServerInstance, TcpServer, UdpServer};
pub use stream_client::StreamClient;
pub use stream_ext::StreamExt;
pub use udp_client::UdpClient;

#[cfg(unix)]
pub use server::UdsServer;

/// rpc client trait
pub trait Client {
    /// call the server
    /// the request must be encoded into the ReqBuf
    /// the response is the raw frame, you should parsing it into final response
    fn call_service(&self, req: ReqBuf) -> Result<Frame, Error>;
}

/// must impl this trait for your server
pub trait Server: Send + Sync + Sized + 'static {
    /// the service that would run in a coroutine
    /// the real request should be deserialized from the input
    /// the real response should be serialized into the RspBuf
    /// if deserialize/serialize error happened, return an Err(WireError)
    /// application error should be encapsulated into the RspBuf
    /// here passed in a self ref to impl stateful service if you want
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError>;
}

/// Provides a few different error types
mod errors;
/// raw frame protocol
mod frame;
mod multiplex_client;
mod queued_writer;
/// Provides server framework
mod server;

/// Provide stream client
mod stream_client;
/// Provides udp client
mod udp_client;

mod stream_ext;
