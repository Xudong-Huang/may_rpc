use std::io::{self, Cursor};
use std::net::ToSocketAddrs;
use std::time::Duration;

use super::errors::Error;
use super::frame::{Frame, ReqBuf};

use bytes::BytesMut;
use may::net::UdpSocket;

/// Udp Client
#[derive(Debug)]
pub struct UdpClient {
    // each request would have a unique id
    id: u64,
    // the connection
    sock: UdpSocket,
    // send/recv buf
    buf: Vec<u8>,
}

impl UdpClient {
    /// connect to the server address
    pub fn connect<L: ToSocketAddrs>(addr: L) -> io::Result<UdpClient> {
        // this would bind a random port by the system
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        sock.connect(addr)?;
        sock.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

        Ok(UdpClient {
            sock,
            id: 0,
            buf: vec![0; 1024],
        })
    }

    /// set the default timeout value
    /// the initial timeout is 1 seconds
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.sock.set_read_timeout(Some(timeout)).unwrap();
    }
}

impl UdpClient {
    /// call the server
    /// the request must be encoded into the ReqBuf
    /// the response is the raw frame, you should parsing it into final response
    pub fn call_service(&mut self, req: ReqBuf) -> Result<Frame, Error> {
        let id = self.id;
        self.id += 1;
        info!("request id = {}", id);

        // send the data to server
        self.sock.send(&(req.finish(id))).map_err(Error::from)?;

        let mut buf = BytesMut::with_capacity(1024 * 32);

        // read the response
        loop {
            self.sock.recv(&mut self.buf).map_err(Error::from)?;

            // deserialize the rsp
            let rsp_frame = Frame::decode_from(&mut Cursor::new(&self.buf), &mut buf)
                .map_err(|e| Error::ClientDeserialize(e.to_string()))?;

            // discard the rsp that is is not belong to us
            if rsp_frame.id == id {
                info!("get response id = {}", id);
                return Ok(rsp_frame);
            }
        }
    }
}
