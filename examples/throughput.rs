use std::sync::Arc;
use std::time::Instant;

#[may_rpc::service]
trait Rpc {
    fn ack(&self, n: usize) -> usize;
}

#[derive(may_rpc::Server)]
#[service(Rpc)]
struct Server;

impl Rpc for Server {
    fn ack(&self, n: usize) -> usize {
        n + 1
    }
}

fn main() {
    use may_rpc::TcpServer;
    let workers = 10000;
    let jobs_per_worker = 1000;

    env_logger::init();
    may::config().set_pool_capacity(10000);
    let addr = ("127.0.0.1", 4000);
    let _server = Server.start(addr).unwrap();
    let clients: Vec<_> = (0..16)
        .map(|_| {
            let stream = may::net::TcpStream::connect(addr).unwrap();
            RpcClient::new(stream).unwrap()
        })
        .collect();
    let clients = Arc::new(clients);
    let mut vec = vec![];
    let now = Instant::now();
    for _i in 0..workers {
        let clients = clients.clone();
        let h = may::go!(move || {
            for j in 0..jobs_per_worker {
                let idx = j & 0x0f;
                match clients[idx].ack(j) {
                    Err(err) => println!("recv err = {:?}", err),
                    Ok(n) => assert_eq!(n, j + 1),
                }
            }
            // println!("thread done, id={_i}");
        });
        vec.push(h);
    }

    for (_i, h) in vec.into_iter().enumerate() {
        h.join().unwrap();
        // println!("thread {_i} joined");
    }

    let dur = now.elapsed();
    let dur = dur.as_secs() as f32 + dur.subsec_nanos() as f32 / 1_000_000_000.0;
    let throughput = workers as f32 * jobs_per_worker as f32 / dur;
    println!("elapsed {dur:?}s, {throughput} rpc/second",);
    std::thread::sleep(std::time::Duration::from_secs(1));
}
