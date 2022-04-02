//! may_rpc is an RPC framework for rust with a focus on ease of use, based on coroutines.
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
//! RPC frameworks are a fundamental building block of most microservice-oriented
//! architectures. Two well-known ones are [gRPC](http://www.grpc.io) and
//! [Cap'n Proto](https://capnproto.org/).
//!
//! may_rpc differentiates itself from other RPC frameworks by defining the schema in code,
//! rather than in a separate language such as .proto. This means there's no separate compilation
//! process, and no cognitive context switching between different languages. Additionally, it
//! works with the community-backed library serde: any serde-serializable type can be used as
//! arguments to may_rpc `fn`s.
//!
//! Example usage:
//!
//! ```rust
//!
//! #[may_rpc::service]
//! trait Hello {
//!     fn hello(name: String) -> String;
//! }
//!
//! #[may_rpc::server]
//! struct HelloServer;
//!
//! impl Hello for HelloServer {
//!     fn hello(&self, name: String) -> String {
//!         format!("Hello, {}!", name)
//!     }
//! }
//!
//! fn main() {
//!     use may_rpc::TcpServer;
//!     let addr = "127.0.0.1:10000";
//!     HelloServer.start(addr).unwrap();
//!
//!     let stream = may::net::TcpStream::connect(addr).unwrap();
//!     let client = HelloClient::new(stream).unwrap();
//!     println!("{}", client.hello("Mom".to_string()).unwrap());
//! }
//! ```
//!

#![deny(missing_docs)]

// re-export conetty
pub use conetty;
// re-export conetty
pub use bincode;
// re-export serde
pub use serde;

// re-export all conetty types
pub use conetty::*;
pub use may_rpc_derive::{server, service};

