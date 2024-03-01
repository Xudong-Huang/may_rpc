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

#![deny(missing_docs)]

#[macro_use]
extern crate log;

mod conetty;

pub use bincode;
pub use serde;

pub use conetty::{
    Client, Error, Frame, MultiplexClient, ReqBuf, RspBuf, Server, ServerInstance, StreamClient,
    StreamExt, TcpServer, UdpClient, UdpServer, UdsServer, WireError,
};
pub use may_rpc_derive::{service, Server};
