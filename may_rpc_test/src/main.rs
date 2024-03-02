mod test_hello_bar;
mod test_hello_foo;

fn test_foo() {
    pub use may_rpc::TcpServer;

    use test_hello_foo::{HelloClient, HelloService};
    let addr = ("127.0.0.1", 4000);

    let service = HelloService;
    let _server = service.start(addr).unwrap();

    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let mut client = HelloClient::new(tcp_stream).unwrap();
    client.set_timeout(::std::time::Duration::from_millis(100));

    for i in 0..10 {
        let s = format!("Hello World! id={}", i);
        let data = client.echo(s);
        println!("recv = {:?}", data);
    }

    for i in 0..10 {
        let data = client.add(i, i);
        println!("recv = {:?}", data);
    }
}

fn test_bar() {
    use may_rpc::TcpServer;
    use test_hello_bar::{HelloClient, HelloService};
    let addr = ("127.0.0.1", 4000);

    let service = HelloService;
    let _server = service.start(addr).unwrap();

    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let mut client = HelloClient::new(tcp_stream).unwrap();
    client.set_timeout(::std::time::Duration::from_millis(100));

    for i in 0..10 {
        let s = format!("Hello World! id={}", i);
        let data = client.echo(s);
        println!("recv = {:?}", data);
    }

    for i in 0..10 {
        let data = client.add(i, i);
        println!("recv = {:?}", data);
    }

    for _i in 0..10 {
        let data = client.xxxx();
        println!("recv = {:?}", data);
    }

    for _i in 0..10 {
        client.yyyy("no return".to_string()).unwrap();
    }
}

fn main() {
    env_logger::init();

    test_foo();
    test_bar();
}
