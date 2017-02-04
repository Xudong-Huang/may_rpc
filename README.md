## corpc: Rust RPC lib based on coroutine
[![Software License](https://img.shields.io/badge/license-MIT-brightgreen.svg)](LICENSE)

corpc is an RPC framework for rust based on coroutines with a focus on ease of use. Inspired by [tarpc](https://github.com/google/tarpc)

[Documentation](https://docs.rs/corpc)

## What is an RPC framework?
"RPC" stands for "Remote Procedure Call," a function call where the work of
producing the return value is being done somewhere else. When an rpc function is
invoked, behind the scenes the function contacts some other process somewhere
and asks them to evaluate the function instead. The original function then
returns the value produced by the other process.

RPC frameworks are a fundamental building block of most microservices-oriented
architectures. Two well-known ones are [gRPC](http://www.grpc.io) and
[Cap'n Proto](https://capnproto.org/).

corpc differentiates itself from other RPC frameworks by defining the schema in code,
rather than in a separate language such as .proto. This means there's no separate compilation
process, and no cognitive context switching between different languages. Additionally, it
works with the community-backed library serde: any serde-serializable type can be used as
arguments to corpc fns.

## Usage

Add to your `Cargo.toml` dependencies:

```toml
corpc = { git = "https://github.com/Xudong-Huang/corpc" }
```

## Example

```rust
#[macro_use]
extern crate corpc;

rpc! {
   rpc hello(name: String) -> String;
}

struct HelloImpl;

impl RpcSpec for HelloImpl {
   fn hello(&self, name: String) -> String {
       format!("Hello, {}!", name)
   }
}

fn main() {
   let addr = "localhost:10000";
   let server = RpcServer(HelloImpl).start(addr).unwrap();
   let client = RpcClient::connect(addr).unwrap();
   println!("{}", client.hello("Mom".to_string()).unwrap());

   // terminate the server
   unsafe { server.coroutine().cancel(); }
   server.join().unwrap()
}
```

The `rpc!` macro expands to a collection of items that form an
rpc service. In the above example, the macro is called within the
`hello_service` module. This module will contain `RpcClient` type,
`RpcServer` type and `RpcSpec` trait. These generated types make
it easy and ergonomic to write servers without dealing with sockets
or serialization directly. Simply implement one of the generated
traits, and you're off to the races!

See the examples directory for more examples.

### Sync vs Async

A single `rpc!` invocation generates code that can be used for both synchronous and asynchronous situations. if you run it in a normal thread context, the thread would be blocked until rpc response come back. if you run in in a coroutine context it will automatically have the ability of non-blocking io.

### Errors

All generated corpc RPC methods return `Result<T, conetty::Error>`. the Error reason could be an io error or timeout.

Default timeout is 10s while you can configure through the RpcClient instance.

## Documentation

Use `cargo doc` as you normally would to see the documentation created for all
items expanded by a `rpc!` invocation.

## Additional Features

- Concurrent requests from a single client. client can be cloned to reuse the connection
- Run any number of clients and services
- Any type that `impl`s `serde`'s `Serialize` and `Deserialize` can be used in
  rpc signatures.
- Attributes can be specified on rpc methods. These will be included on both the
  services' trait methods as well as on the clients' stub methods.

## License

corpc is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
