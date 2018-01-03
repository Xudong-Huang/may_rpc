# may_rpc

Rust coroutine based RPC framework

[![Build Status](https://travis-ci.org/Xudong-Huang/may_rpc.svg?branch=master)](https://travis-ci.org/Xudong-Huang/may_rpc)
[![Build status](https://ci.appveyor.com/api/projects/status/a2y8e6f8h2r49l1d/branch/master?svg=true)](https://ci.appveyor.com/project/Xudong-Huang/may-rpc/branch/master)
[![Software License](https://img.shields.io/badge/license-MIT-brightgreen.svg)](LICENSE)

may_rpc is an RPC framework for rust based on coroutines that powered by [may](https://github.com/Xudong-Huang/may) with a focus on ease of use. Inspired by [tarpc](https://github.com/google/tarpc).

## Usage

Add to your `Cargo.toml` dependencies:

```toml
may_rpc = { git = "https://github.com/Xudong-Huang/may_rpc" }
```

## Example

```rust
#[macro_use]
extern crate may_rpc;
#[macro_use]
extern crate serde_derive;

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

The `rpc!` macro expands to a collection of items that form an rpc service. In the above example, the `rcp!` macro is called with a **rcp spec**. This will generate `RpcClient` type, `RpcServer` type and `RpcSpec` trait. These generated types make it easy and ergonomic to write servers without dealing with sockets or serialization directly. Simply implement the generated traits, and you're off to the races! 

See the examples directory for more examples.

### Errors

All generated may_rpc RPC methods return `Result<T, conetty::Error>`. the Error reason could be an io error or timeout. 

Default timeout is 10s while you can configure through the RpcClient instance.

## Performance

Just run the throughput example under this project

**Machine Specs:**

  * **Logical Cores:** 4 (4 cores x 2 threads)
  * **Memory:** 4gb ECC DDR3 @ 1600mhz
  * **Processor:** CPU Intel(R) Core(TM) i7-3820QM CPU @ 2.70GHz
  * **Operating System:** Windows 10

**Test config:**
```rust
may::config().set_workers(6).set_io_workers(4);
```
result:

```sh
$ cargo run --example=throughput --release
......
     Running `target\release\examples\throughput.exe`
206127.39 rpc/second
```

## Additional Features

- Concurrent requests from a single client. client can be cloned to reuse the connection
- Run any number of clients and services
- Any type that `impl`s `serde`'s `Serialize` and `Deserialize` can be used in
  rpc signatures.
- Attributes can be specified on rpc methods. These will be included on both the
  services' trait methods as well as on the clients' stub methods.

## License

may_rpc is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
