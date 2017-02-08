#[macro_use]
extern crate corpc;
extern crate env_logger;

// cargo rustc --bin main -- -Z unstable-options --pretty expanded

rpc! {
    /// the connection type, default is Tcp
    net: Udp;
    /// Say hello
    rpc hello(name: String) -> String;
    /// add two number
    rpc add(x: u32, y: u32 ) -> u32;
}

mod count {
    rpc! {
        /// get current count
        rpc get_count() -> usize;
    }
}

fn test_count() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use count;
    struct CountImpl(AtomicUsize);
    impl count::RpcSpec for CountImpl {
        fn get_count(&self) -> usize {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    let addr = ("127.0.0.1", 4000);
    let server = count::RpcServer(CountImpl(AtomicUsize::new(0))).start(&addr).unwrap();
    let mut client = count::RpcClient::connect(addr).unwrap();
    client.set_timeout(::std::time::Duration::from_millis(100));

    for _ in 0..10 {
        let data = client.get_count();
        println!("recv = {:?}", data);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

fn test_hello() {
    struct HelloImpl;
    impl RpcSpec for HelloImpl {
        fn hello(&self, name: String) -> String {
            name
        }

        fn add(&self, x: u32, y: u32) -> u32 {
            x + y
        }
    }

    let addr = ("127.0.0.1", 4000);
    let server = RpcServer(HelloImpl).start(&addr).unwrap();
    let client = RpcClient::connect(addr).unwrap();

    for i in 0..10 {
        let s = format!("Hello World! id={}", i);
        let data = client.hello(s);
        println!("recv = {:?}", data);
    }

    for i in 0..10 {
        let data = client.add(i, i);
        println!("recv = {:?}", data);
    }

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

fn main() {
    env_logger::init().unwrap();
    corpc::conetty::coroutine::scheduler_config().set_workers(2).set_io_workers(4);

    println!("test_hello");
    test_hello();

    println!("\n\ntest_count");
    test_count();
}
