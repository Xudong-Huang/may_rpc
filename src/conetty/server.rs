use std::io::{self, BufReader, Cursor};
use std::net::ToSocketAddrs;
#[cfg(unix)]
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::frame::{Frame, RspBuf};
use super::queued_writer::QueuedWriter;
use crate::Server;

use bytes::BytesMut;
use co_managed::Manager;
use may::net::{TcpListener, UdpSocket};
#[cfg(unix)]
use may::os::unix::net::UnixListener;
use may::sync::Mutex;
use may::{coroutine, go};

macro_rules! t {
    ($e: expr) => {
        match $e {
            Ok(val) => val,
            Err(err) => {
                error!("call = {:?}\nerr = {:?}", stringify!($e), err);
                continue;
            }
        }
    };
}

/// service instance
pub struct ServerInstance(Option<coroutine::JoinHandle<()>>);

impl ServerInstance {
    /// join the service, this would wait until the service is stopped
    pub fn join(mut self) -> std::thread::Result<()> {
        if let Some(handle) = self.0.take() {
            handle.join()
        } else {
            Ok(())
        }
    }
}

impl Drop for ServerInstance {
    fn drop(&mut self) {
        if let Some(s) = self.0.take() {
            unsafe { s.coroutine().cancel() };
            s.join().ok();
        }
    }
}

/// Provides a function for starting the service.
pub trait UdpServer: Server {
    /// Spawns the service, binding to the given address
    /// return a coroutine that you can cancel it when need to stop the service
    fn start<L: ToSocketAddrs>(self, addr: L) -> io::Result<ServerInstance> {
        let sock = UdpSocket::bind(addr)?; // the write half
        let sock1 = sock.try_clone()?; // the read half
        let instance = go!(
            coroutine::Builder::new().name("UdpServer".to_owned()),
            move || {
                let server = Arc::new(self);
                let mut buf = vec![0u8; 1024];
                // the write half need to be protected by mutex
                // for that coroutine io obj can't shared safely
                let sock = Arc::new(Mutex::new(sock));
                let mut body_buf = BytesMut::with_capacity(1024 * 32);
                loop {
                    // each udp packet should be less than 1024 bytes
                    let (len, addr) = t!(sock1.recv_from(&mut buf));
                    info!("recv_from: len={:?} addr={:?}", len, addr);

                    // if we failed to deserialize the request frame, just continue
                    let req = t!(Frame::decode_from(&mut Cursor::new(&buf), &mut body_buf));
                    let sock = sock.clone();
                    let server = server.clone();
                    // let mutex = mutex.clone();
                    go!(move || {
                        let mut rsp = RspBuf::new();
                        let ret = server.service(req.decode_req(), &mut rsp);
                        let data = rsp.finish(req.id, ret);

                        info!("send_to: len={:?} addr={:?}", data.len(), addr);

                        // send the result back to client
                        // udp no need to protect by a mutex, each send would be one frame
                        let s = sock.lock().unwrap();
                        match s.send_to(&data, addr) {
                            Ok(_) => {}
                            Err(err) => error!("udp send_to failed, err={:?}", err),
                        }
                    });
                }
            }
        )?;
        Ok(ServerInstance(Some(instance)))
    }
}

/// Provides a function for starting the tcp service.
pub trait TcpServer: Server {
    /// Spawns the service, binding to the given address
    /// return a coroutine that you can cancel it when need to stop the service
    fn start<L: ToSocketAddrs>(self, addr: L) -> io::Result<ServerInstance> {
        let listener = TcpListener::bind(addr)?;
        let instance = go!(
            coroutine::Builder::new().name("TcpServer".to_owned()),
            move || {
                let server = Arc::new(self);
                let manager = Manager::new();
                for stream in listener.incoming() {
                    let stream = t!(stream);
                    stream.set_nodelay(true).unwrap();
                    let server = server.clone();
                    manager.add(move || {
                        let rs = stream.try_clone().expect("failed to clone stream");
                        // the read half of the stream
                        let mut rs = BufReader::new(rs);
                        // the write half of the stream
                        let ws = Arc::new(QueuedWriter::new(stream));
                        let mut buf = BytesMut::with_capacity(1024 * 32);
                        loop {
                            let req = match Frame::decode_from(&mut rs, &mut buf) {
                                Ok(r) => r,
                                Err(ref e) => {
                                    if e.kind() == io::ErrorKind::UnexpectedEof {
                                        info!("tcp server decode req: connection closed");
                                    } else {
                                        error!("tcp server decode req: err = {:?}", e);
                                    }
                                    break;
                                }
                            };

                            info!("get request: id={:?}", req.id);
                            let w_stream = ws.clone();
                            let server = server.clone();
                            go!(move || {
                                let mut rsp = RspBuf::new();
                                let ret = server.service(req.decode_req(), &mut rsp);
                                let data = rsp.finish(req.id, ret);

                                info!("send rsp: id={}", req.id);
                                // send the result back to client
                                w_stream.write(data).expect("tcp write to client failed");
                            });
                        }
                    });
                }
            }
        )?;
        Ok(ServerInstance(Some(instance)))
    }
}

/// Provides a function for starting the unix domain socket service.
#[cfg(unix)]
pub trait UdsServer: Server {
    /// Spawns the service, binding to the given address
    /// return a coroutine that you can cancel it when need to stop the service
    fn start<P: AsRef<Path>>(self, path: P) -> io::Result<ServerInstance> {
        struct AutoDrop(UnixListener, PathBuf);
        impl Drop for AutoDrop {
            fn drop(&mut self) {
                std::fs::remove_file(&self.1).ok();
            }
        }

        std::fs::remove_file(&path).ok();
        let listener = AutoDrop(UnixListener::bind(&path)?, path.as_ref().to_owned());
        let instance = go!(
            coroutine::Builder::new().name("Unix Socket Server".to_owned()),
            move || {
                let server = Arc::new(self);
                let manager = Manager::new();
                for stream in listener.0.incoming() {
                    let stream = t!(stream);
                    let server = server.clone();
                    manager.add(move || {
                        let rs = stream.try_clone().expect("failed to clone stream");
                        // the read half of the stream
                        let mut rs = BufReader::new(rs);
                        // the write half need to be protected by mutex
                        // for that coroutine io obj can't shared safely
                        let ws = Arc::new(QueuedWriter::new(stream));
                        let mut buf = BytesMut::with_capacity(1024 * 32);
                        loop {
                            let req = match Frame::decode_from(&mut rs, &mut buf) {
                                Ok(r) => r,
                                Err(ref e) => {
                                    if e.kind() == io::ErrorKind::UnexpectedEof {
                                        info!("uds server decode req: connection closed");
                                    } else {
                                        error!("uds server decode req: err = {:?}", e);
                                    }
                                    break;
                                }
                            };

                            info!("get request: id={:?}", req.id);
                            let w_stream = ws.clone();
                            let server = server.clone();
                            go!(move || {
                                let mut rsp = RspBuf::new();
                                let ret = server.service(req.decode_req(), &mut rsp);
                                let data = rsp.finish(req.id, ret);

                                info!("send rsp: id={}", req.id);
                                // send the result back to client
                                w_stream.write(data).expect("uds write to client failed");
                            });
                        }
                    });
                }
            }
        )?;
        Ok(ServerInstance(Some(instance)))
    }
}

impl<T: Server> UdpServer for T {}
impl<T: Server> TcpServer for T {}
#[cfg(unix)]
impl<T: Server> UdsServer for T {}
