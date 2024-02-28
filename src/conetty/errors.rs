use std::io;

use thiserror::Error;

/// All errors that can occur during the use of tarpc.
#[derive(Debug, Error)]
pub enum Error {
    /// Any IO error.
    #[error("IO err: {0}")]
    Io(#[from] io::Error),
    /// Error in deserializing a server response.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize` or
    /// `serde::Deserialize`.
    #[error("deserializing a server response err: {0}")]
    ClientDeserialize(String),
    /// Error in serializing a client request.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize`.
    #[error("serializing a client request err: {0}")]
    ClientSerialize(String),
    /// Error in deserializing a client request.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize` or
    /// `serde::Deserialize`.
    #[error("deserializing a client request err: {0}")]
    ServerDeserialize(String),
    /// Error in serializing a server response.
    ///
    /// Typically this indicates a faulty implementation of `serde::Serialize`.
    #[error("serializing a server response err: {0}")]
    ServerSerialize(String),
    /// The server was unable to reply to the rpc client within some time.
    ///
    /// You can set the default timeout value in the client instance
    #[error("The server was unable to reply to the rpc client within some time")]
    Timeout,
    /// The server returns an status error due to different reasons.
    ///
    /// Typically this indicates that the server is not healthy
    #[error("The server returns an status error due to different reasons: {0}")]
    Status(String),
}

/// A serializable, server-supplied error.
#[doc(hidden)]
#[derive(Debug, Error)]
pub enum WireError {
    #[error("Deserializing a client request: {0}")]
    ServerDeserialize(String),
    #[error("Serializing server response: {0}")]
    ServerSerialize(String),
    /// Server Status
    #[error("Server Status: {0}")]
    Status(String),
    /// Server polling
    /// this is a special error code that used for server polling request from client
    /// client will first check this code in the very beginning before return to client rpc call
    #[error("Server polling")]
    Polling,
}
