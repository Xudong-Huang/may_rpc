#![feature(test)]
#[macro_use]
extern crate corpc;
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
extern crate test;
#[cfg(test)]
use test::Bencher;

rpc! {
    rpc ack();
}

struct Server;

impl RpcSpec for Server {
    fn ack(&self) {}
}

#[cfg(test)]
#[bench]
fn latency(bencher: &mut Bencher) {
    let addr = ("127.0.0.1", 4000);
    let server = RpcServer(Server).start(&addr).unwrap();
    let client = RpcClient::connect(addr).unwrap();

    bencher.iter(|| {
        client.ack().unwrap();
    });

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
