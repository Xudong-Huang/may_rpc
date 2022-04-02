mod test_hello_foo;
// mod test_hello_bar;

fn test_foo() {
    use conetty::TcpServer;
    use test_hello_foo::{/*Hello, */ HelloClient, HelloService};
    let addr = ("127.0.0.1", 4000);

    let service = HelloService;
    let server = service.start(&addr).unwrap();

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

    unsafe { server.coroutine().cancel() };
    server.join().ok();
}

// fn test_bar() {
// 	use test_hello_bar::{Hello, HelloClient, HelloServer};
// 	let addr = ("127.0.0.1", 4000);
//     let server = HelloServer::start(&addr).unwrap();

//     let mut client = HelloClient::connect(addr).unwrap();
//     client.set_timeout(::std::time::Duration::from_millis(100));

//     for i in 0..10 {
//         let s = format!("Hello World! id={}", i);
//         let data = client.echo(s);
//         println!("recv = {:?}", data);
//     }

//     for i in 0..10 {
//         let data = client.add(i, i);
//         println!("recv = {:?}", data);
//     }

//     unsafe { server.coroutine().cancel() };
//     server.join().ok();
// }

fn main() {
    env_logger::init();

    // test_bar();
    test_foo();
}
