# may_rpc

Rust coroutine based RPC framework

[![Build Status](https://github.com/Xudong-Huang/may_rpc/workflows/CI/badge.svg)](https://github.com/Xudong-Huang/may_rpc/actions?query=workflow%3ACI)
[![Current Crates.io Version](https://img.shields.io/crates/v/may_rpc.svg)](https://crates.io/crates/may_rpc)
[![Document](https://img.shields.io/badge/doc-may_rpc-green.svg)](https://docs.rs/may_rpc)

may_rpc is an RPC framework for rust based on coroutines that powered by [may](https://github.com/Xudong-Huang/may) with a focus on ease of use. Inspired by [tarpc](https://github.com/google/tarpc).

## Usage

Add to your `Cargo.toml` dependencies:

```toml
may_rpc = "0.1"
```

## Example

```rust
#[may_rpc::service]
trait Hello {
    fn hello(&self, name: String) -> String;
}

#[derive(may_rpc::Server)]
#[service(Hello)]
struct HelloServer;

impl Hello for HelloServer {
    fn hello(&self, name: String) -> String {
        format!("Hello, {}!", name)
    }
}

fn main() {
    use may_rpc::TcpServer;
    let addr = "127.0.0.1:10000";
    let server = HelloServer.start(addr).unwrap();

    let stream = may::net::TcpStream::connect(addr).unwrap();
    let client = HelloClient::new(stream).unwrap();
    println!("{}", client.hello("Mom".to_string()).unwrap());

    server.shutdown();
}
```

The `service` attribute macro expands to a collection of items that form an rpc service. In the above example, the `service` macro is derived for a  **Hello** rcp spec trait. This will generate `HelloClient` for ease of use. Then the `HelloServer` type derive `Server` and impl `Hello` trait for a rpc server. The generated types make it easy and ergonomic to write servers without dealing with sockets or serialization directly. Simply implement the generated traits, and you're off to the races!

See the examples directory for more examples.

### Errors

All generated may_rpc RPC methods return `Result<T, may_rpc::Error>`. the Error reason could be an io error or timeout.

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

This project is licensed under either of the following, at your option:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT).
