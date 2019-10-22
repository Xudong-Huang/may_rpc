#[macro_use]
extern crate may;
#[macro_use]
extern crate may_rpc;
extern crate env_logger;

use std::sync::Arc;
use std::time::Instant;

rpc! {
    net: Multiplex;
    rpc ack(n: usize) -> usize;
}

struct Server;

impl RpcSpec for Server {
    fn ack(&self, n: usize) -> usize {
        n + 1
    }
}

fn main() {
    env_logger::init();
    may::config().set_pool_capacity(10000);
    let addr = ("127.0.0.1", 4000);
    let server = RpcServer(Server).start(&addr).unwrap();
    let clients: Vec<_> = (0..16).map(|_| RpcClient::connect(addr).unwrap()).collect();
    let clients = Arc::new(clients);
    let mut vec = vec![];
    let now = Instant::now();
    for _i in 0..1000 {
        let clients = clients.clone();
        let h = go!(move || {
            for j in 0..10000 {
                let idx = j & 0x0f;
                match clients[idx].ack(j) {
                    Err(err) => println!("recv err = {:?}", err),
                    Ok(n) => assert_eq!(n, j + 1),
                }
            }
            // println!("thread done, id={}", i);
        });
        vec.push(h);
    }

    for h in vec {
        h.join().unwrap();
    }

    let dur = now.elapsed();
    let dur = dur.as_secs() as f32 + dur.subsec_nanos() as f32 / 1000_000_000.0;
    println!("{} rpc/second", 10_000_000.0 / dur);

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
