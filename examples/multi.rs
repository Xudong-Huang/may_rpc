use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[may_rpc::service]
trait RpcSpec {
    /// get current count
    fn get_count(&self) -> usize;
}

#[derive(may_rpc::Server)]
#[service(RpcSpec)]
struct CountImpl(AtomicUsize);

impl RpcSpec for CountImpl {
    fn get_count(&self) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

fn main() {
    use may_rpc::TcpServer;
    let addr = ("127.0.0.1", 4000);
    let _server = CountImpl(AtomicUsize::new(0)).start(addr).unwrap();

    let stream = may::net::TcpStream::connect(addr).unwrap();
    let client = Arc::new(RpcSpecClient::new(stream).unwrap());

    let mut vec = vec![];
    for i in 0..100 {
        let client = client.clone();
        let j = may::go!(move || {
            for _j in 0..1000 {
                if let Err(err) = client.get_count() {
                    // Ok(data) => println!("recv = {:?}", str::from_utf8(&data).unwrap()),
                    println!("recv err = {:?}", err);
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
}
