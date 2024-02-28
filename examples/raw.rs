use std::io::Write;
use std::str;

use may_rpc::{ReqBuf, RspBuf, Server, StreamClient, TcpServer, WireError};

struct Echo;

impl Server for Echo {
    fn service(&self, req: &[u8], rsp: &mut RspBuf) -> Result<(), WireError> {
        println!("req = {req:?}");
        rsp.write_all(req)
            .map_err(|e| WireError::ServerSerialize(e.to_string()))
    }
}

fn main() {
    let addr = ("127.0.0.1", 4000);
    let _server = Echo.start(addr).unwrap();
    let tcp_stream = may::net::TcpStream::connect(addr).unwrap();
    let mut client = StreamClient::new(tcp_stream);

    for i in 0..10 {
        let mut buf = ReqBuf::new();
        buf.write_fmt(format_args!("Hello World! id={i}")).unwrap();
        let data = client.call_service(buf).unwrap();
        let rsp = data.decode_rsp().unwrap();
        println!("recv = {:?}", str::from_utf8(rsp).unwrap());
    }
}
