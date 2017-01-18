//! corpc is an RPC framework for rust with a focus on ease of use, based on coroutines.
//! Defining a service can be done in just a few lines of code, and most of the boilerplate of
//! writing a server is taken care of for you.
//!
//! ## What is an RPC framework?
//! "RPC" stands for "Remote Procedure Call," a function call where the work of
//! producing the return value is being done somewhere else. When an rpc function is
//! invoked, behind the scenes the function contacts some other process somewhere
//! and asks them to evaluate the function instead. The original function then
//! returns the value produced by the other process.
//!
//! RPC frameworks are a fundamental building block of most microservices-oriented
//! architectures. Two well-known ones are [gRPC](http://www.grpc.io) and
//! [Cap'n Proto](https://capnproto.org/).
//!
//! corpc differentiates itself from other RPC frameworks by defining the schema in code,
//! rather than in a separate language such as .proto. This means there's no separate compilation
//! process, and no cognitive context switching between different languages. Additionally, it
//! works with the community-backed library serde: any serde-serializable type can be used as
//! arguments to corpc fns.
//!
//! Example usage:
//!
//! ```rust,ignore
//! #[macro_use]
//! extern crate corpc;
//!
//! use corpc::{client, server};
//!
//! rpc_service! {
//!     rpc hello(name: String) -> String;
//! }
//!
//! #[derive(Clone)]
//! struct HelloServer;
//!
//! impl RpcService for HelloServer {
//!     fn hello(&self, name: String) -> String {
//!         format!("Hello, {}!", name)
//!     }
//! }
//!
//! fn main() {
//!     let addr = "localhost:10000";
//!     HelloServer.listen(addr).unwrap();
//!     let client = RpcClient::connect(addr).unwrap();
//!     println!("{}", client.hello("Mom".to_string()).unwrap());
//! }
//! ```
//!

#![deny(missing_docs)]
// extern crate byteorder;
// extern crate bytes;
// #[macro_use]
// extern crate log;
// extern crate net2;
#[doc(hidden)]
pub extern crate serde;
#[macro_use]
extern crate serde_derive;
// extern crate take;

// #[doc(hidden)]
// pub extern crate bincode;
// #[doc(hidden)]
// pub extern crate serde;
//
// #[doc(hidden)]
// pub use client::Client;
pub use errors::Error;
#[doc(hidden)]
pub use errors::WireError;

/// Provides some utility error types, as well as a trait for spawning futures on the default event
/// loop.
// pub mod util;
/// Provides the macro used for constructing rpc services and client stubs.
// #[macro_use]
// mod macros;
/// Provides the base client stubs used by the service macro.
// pub mod client;
/// Provides the base server boilerplate used by service implementations.
// pub mod server;
/// Provides implementations of `ClientProto` and `ServerProto` that implement the tarpc protocol.
/// The tarpc protocol is a length-delimited, bincode-serialized payload.
// mod protocol;
/// Provides a few different error types.
mod errors;
