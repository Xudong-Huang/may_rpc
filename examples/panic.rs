#[may_rpc::service]
trait RpcSpec {
    fn add(&self, x: u32, y: u32) -> u32;
}

#[derive(may_rpc::Server)]
#[service(RpcSpec)]
struct RcpServer;
impl RpcSpec for RcpServer {
    fn add(&self, _x: u32, _y: u32) -> u32 {
        panic!("painc in side")
    }
}

fn main() {
    use may_rpc::conetty::TcpServer;
    let addr = ("127.0.0.1", 4000);
    let server = RcpServer.start(&addr).unwrap();

    let stream = may::net::TcpStream::connect(&addr).unwrap();
    let client = RpcSpecClient::new(stream).unwrap();
    println!("rsp = {:?}", client.add(1, 4));
    // assert_eq!(client.add(1, 4).is_err(), true);
    println!("done");
    server.shutdown();
}
