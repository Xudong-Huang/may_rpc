#[macro_use]
extern crate may;
#[macro_use]
extern crate may_rpc;
#[macro_use]
extern crate serde_derive;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

rpc! {
    net: Multiplex;
    /// get current count
    rpc get_count() -> usize;
}

struct CountImpl(AtomicUsize);

impl RpcSpec for CountImpl {
    fn get_count(&self) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

fn main() {
    let addr = ("127.0.0.1", 4000);
    let server = RpcServer(CountImpl(AtomicUsize::new(0)))
        .start(&addr)
        .unwrap();
    may::config().set_workers(2).set_io_workers(2);
    let client = Arc::new(RpcClient::connect(addr).unwrap());

    let mut vec = vec![];
    for i in 0..100 {
        let client = client.clone();
        let j = go!(move || {
            for _j in 0..1000 {
                match client.get_count() {
                    // Ok(data) => println!("recv = {:?}", str::from_utf8(&data).unwrap()),
                    Err(err) => println!("recv err = {:?}", err),
                    _ => {}
                }
            }
            println!("thread done, id={}", i);
        });
        vec.push(j);
    }

    for (i, j) in vec.into_iter().enumerate() {
        j.join().unwrap();
        println!("wait for {} done", i);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}
