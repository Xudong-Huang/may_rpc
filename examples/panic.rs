#[macro_use]
extern crate may_rpc;

rpc! {
    rpc add(x: u32, y: u32) -> u32;
}

struct HelloImpl;
impl RpcSpec for HelloImpl {
    fn add(&self, _x: u32, _y: u32) -> u32 {
        panic!("painc in side")
    }
}

fn main() {
    let addr = ("127.0.0.1", 4000);
    let _server = RpcServer(HelloImpl).start(&addr).unwrap();
    let client = RpcClient::connect(addr).unwrap();
    println!("rsp = {:?}", client.add(1, 4));
    // assert_eq!(client.add(1, 4).is_err(), true);
}
