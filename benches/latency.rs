#![feature(test)]

#[cfg(test)]
extern crate test;
#[cfg(test)]
use test::Bencher;

#[may_rpc::service]
trait RpcSpec {
    fn ack(&self);
}

#[derive(may_rpc::Server)]
#[service(RpcSpec)]
struct Server;

impl RpcSpec for Server {
    fn ack(&self) {}
}

#[cfg(test)]
#[bench]
fn latency(bencher: &mut Bencher) {
    use may_rpc::conetty::TcpServer;
    let addr = ("127.0.0.1", 4000);
    let _server = Server.start(addr).unwrap();
    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let client = RpcSpecClient::new(tcp_stream).unwrap();

    bencher.iter(|| {
        client.ack().unwrap();
    });
}
