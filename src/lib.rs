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
//! ```rust
//! #[macro_use]
//! extern crate corpc;
//!
//! rpc! {
//!     rpc hello(name: String) -> String;
//! }
//!
//! struct HelloImpl;
//!
//! impl RpcSpec for HelloImpl {
//!     fn hello(&self, name: String) -> String {
//!         format!("Hello, {}!", name)
//!     }
//! }
//!
//! fn main() {
//!     let addr = "localhost:10000";
//!     RpcServer(HelloImpl).start(addr).unwrap();
//!     let client = RpcClient::connect(addr).unwrap();
//!     println!("{}", client.hello("Mom".to_string()).unwrap());
//! }
//! ```
//!

#![deny(missing_docs)]
#![feature(macro_reexport)]

#[doc(hidden)]
pub extern crate conetty;
#[doc(hidden)]
pub extern crate bincode;

#[allow(unused)]
#[macro_use]
#[macro_reexport(Serialize, Deserialize)]
extern crate serde_derive;

/// dispatch rpc client according to connection type
#[macro_export]
macro_rules! rpc_client {
    (Tcp) => {$crate::conetty::TcpClient};
    (Udp) => {$crate::conetty::UdpClient};
    (Multiplex) => {$crate::conetty::MultiplexClient};
}

/// dispatch rpc server according to connection type
#[macro_export]
macro_rules! rpc_server_start {
    (Tcp, $me: ident, $addr: expr) => {$crate::conetty::TcpServer::start($me, $addr)};
    (Udp, $me: ident, $addr: expr) => {$crate::conetty::UdpServer::start($me, $addr)};
    (Multiplex, $me: ident, $addr: expr) => {$crate::conetty::TcpServer::start($me, $addr)};
}


/// The main macro that creates RPC services.
///
/// Rpc methods are specified, mirroring trait syntax:
///
/// ```rust
/// # #[macro_use] extern crate corpc;
/// # #[macro_use] extern crate serde_derive;
/// # fn main() {}
/// rpc! {
///     /// Say hello
///     rpc hello(name: String) -> String;
/// }
/// ```
///
/// Attributes can be attached to each rpc. These attributes
/// will then be attached to the generated rpc spec traits'
/// corresponding `fn`s, as well as to the client stubs' RPCs.
///
/// The following items are expanded in the enclosing module:
///
/// * `RpcSpec`   -- the trait defining the RPC service that user need to impl
/// * `RpcServer` -- the server that run the RPC service
/// * `RpcClient` -- rpc client stubs implementation that wrap UdpClient/TcpClient
///
///  Usable net types are `Tcp`, `Udp`, `Multiplex`, please ref `conetty`
///
#[macro_export]
macro_rules! rpc {
// Entry point without net
    (
        $(
            $(#[$attr:meta])*
            rpc $fn_name:ident( $( $arg:ident : $in_:ty ),* ) $(-> $out:ty)*;
        )*
    ) => {
        rpc! {
            net: Tcp;
            $(
                $(#[$attr])*
                rpc $fn_name( $( $arg : $in_ ),* ) $(-> $out)*;
            )*
        }
    };
// Entry point with net
    (
        $(#[$net_attr:meta])*
        net: $net_type: ident;
        $(
            $(#[$attr:meta])*
            rpc $fn_name:ident( $( $arg:ident : $in_:ty ),* ) $(-> $out:ty)*;
        )*
    ) => {
        rpc! {
            $(#[$net_attr])*
            net: $net_type;
            {
                $(
                    $(#[$attr])*
                    rpc $fn_name( $( $arg : $in_ ),* ) $(-> $out)*;
                )*
            }
        }
    };
// Pattern for when the next rpc has an implicit unit return type
    (
        $(#[$net_attr:meta])*
        net: $net_type: ident;
        {
            $(#[$attr:meta])*
            rpc $fn_name:ident( $( $arg:ident : $in_:ty ),* );

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        rpc! {
            $(#[$net_attr])*
            net: $net_type;
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            rpc $fn_name( $( $arg : $in_ ),* ) -> ();
        }
    };
// Pattern for when the next rpc has an explicit return type and an explicit error type.
    (
        $(#[$net_attr:meta])*
        net: $net_type: ident;
        {
            $(#[$attr:meta])*
            rpc $fn_name:ident( $( $arg:ident : $in_:ty ),* ) -> $out:ty;

            $( $unexpanded:tt )*
        }
        $( $expanded:tt )*
    ) => {
        rpc! {
            $(#[$net_attr])*
            net: $net_type;
            { $( $unexpanded )* }

            $( $expanded )*

            $(#[$attr])*
            rpc $fn_name( $( $arg : $in_ ),* ) -> $out;
        }
    };
// Pattern for when all return types have been expanded
    (
        $(#[$net_attr:meta])*
        net: $net_type: ident;
        { } // none left to expand
        $(
            $(#[$attr:meta])*
            rpc $fn_name:ident ( $( $arg:ident : $in_:ty ),* ) -> $out:ty;
        )*
    ) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Serialize, Deserialize)]
        enum RpcEnum {
            $(
                $fn_name(( $($in_,)* ))
            ),*
        }

        pub struct RpcClient(rpc_client!($net_type));

        impl RpcClient {
            pub fn connect<L: ::std::net::ToSocketAddrs>(addr: L) -> ::std::io::Result<RpcClient> {
                type Client = rpc_client!($net_type);
                Client::connect(addr).map(RpcClient)
            }

            pub fn set_timeout(&mut self, timeout: ::std::time::Duration) {
                self.0.set_timeout(timeout)
            }

            $(
            pub fn $fn_name(&self, $($arg: $in_),*) -> Result<$out, $crate::conetty::Error> {
                use $crate::conetty::Client;
                use $crate::bincode::serde as encode;
                use $crate::bincode::SizeLimit::Infinite;
                use $crate::conetty::Error::{ClientSerialize, ClientDeserialize};

                let mut buf = Vec::with_capacity(1024);

                // serialize the para
                let para = RpcEnum::$fn_name(($($arg,)*));
                encode::serialize_into(&mut buf, &para, Infinite)
                    .map_err(|e| ClientSerialize(e.to_string()))?;

                // call the server
                let ret = self.0.call_service(&buf)?;

                // deserialized the response
                encode::deserialize(&ret).map_err(|e| ClientDeserialize(e.to_string()))
            })*
        }

        // rpc spec
        pub trait RpcSpec: Send + Sync + 'static {
            $(fn $fn_name(&self, $($arg: $in_),*) -> $out;)*
        }

        // rpc server
        pub struct RpcServer<T>(pub T);

        impl<T: RpcSpec> ::std::ops::Deref for RpcServer<T> {
            type Target = T;
            fn deref(&self) -> &T {
                &self.0
            }
        }

        impl<T: RpcSpec> $crate::conetty::Server for RpcServer<T> {
            fn service(&self, request: &[u8]) -> Result<Vec<u8>, $crate::conetty::WireError> {
                use $crate::bincode::serde as encode;
                use $crate::bincode::SizeLimit::Infinite;
                use $crate::conetty::WireError::{ServerDeserialize, ServerSerialize};

                // deserialize the request
                let req: RpcEnum = encode::deserialize(request)
                    .map_err(|e| ServerDeserialize(e.to_string()))?;
                // dispatch call the service
                let mut buf = Vec::with_capacity(512);
                match req {
                    $(
                    RpcEnum::$fn_name(($($arg,)*)) => {
                        let rsp = self.$fn_name($($arg,)*);
                        // serialize the result
                        encode::serialize_into(&mut buf, &rsp, Infinite)
                            .map_err(|e| ServerSerialize(e.to_string()))?;
                    })*
                };
                // send the response
                Ok(buf)
            }
        }

        impl<T: RpcSpec + 'static> RpcServer<T> {
            pub fn start<L: ::std::net::ToSocketAddrs>(self, addr: L)
                 -> ::std::io::Result<$crate::conetty::coroutine::JoinHandle<()>> {
                rpc_server_start!($net_type, self, addr)
            }
        }
    };
}
