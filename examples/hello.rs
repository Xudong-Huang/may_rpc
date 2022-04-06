// cargo rustc --bin main -- -Z unstable-options --pretty expanded
#[may_rpc::service]
trait RpcSpec {
    /// Say hello
    fn hello(name: String) -> String;
    /// add two number
    fn add(x: u32, y: u32) -> u32;
}

mod count {
    #[may_rpc::service]
    pub trait RpcSpec {
        /// get current count
        fn get_count() -> usize;
    }
}

fn test_count() {
    use may_rpc::conetty::TcpServer;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(may_rpc::Server)]
    #[service(count::RpcSpec)]
    struct CountImpl(AtomicUsize);
    impl count::RpcSpec for CountImpl {
        fn get_count(&self) -> usize {
            self.0.fetch_add(1, Ordering::Relaxed)
        }
    }

    let addr = ("127.0.0.1", 4000);
    let server = CountImpl(AtomicUsize::new(0)).start(&addr).unwrap();

    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let mut client = count::RpcSpecClient::new(tcp_stream).unwrap();
    client.set_timeout(::std::time::Duration::from_millis(100));

    for _ in 0..10 {
        let data = client.get_count();
        println!("recv = {:?}", data);
    }

    server.shutdown();
}

fn test_hello() {
    use may_rpc::conetty::TcpServer;

    #[derive(may_rpc::Server)]
    #[service(RpcSpec)]
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
    let server = HelloImpl.start(&addr).unwrap();
    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let client = RpcSpecClient::new(tcp_stream).unwrap();

    for i in 0..10 {
        let s = format!("Hello World! id={}", i);
        let data = client.hello(s);
        println!("recv = {:?}", data);
    }

    for i in 0..10 {
        let data = client.add(i, i);
        println!("recv = {:?}", data);
    }

    server.shutdown();
}

fn main() {
    env_logger::init();

    println!("test_hello");
    test_hello();

    println!("\n\ntest_count");
    test_count();
}
