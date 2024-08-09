pub trait Hello {
    fn echo(&self, data: String) -> String;
    fn add(&self, x: u32, y: u32) -> u32;
}

#[derive(Debug, bitcode::Encode, bitcode::Decode)]
pub enum HelloRequest {
    Echo { data: String },
    Add { x: u32, y: u32 },
}

#[derive(Debug)]
pub struct HelloClient<S: may_rpc::StreamExt> {
    transport: may_rpc::MultiplexClient<S>,
}

impl<S: may_rpc::StreamExt> HelloClient<S> {
    pub fn new(stream: S) -> std::io::Result<Self> {
        let transport = may_rpc::MultiplexClient::new(stream)?;
        Ok(Self { transport })
    }
    pub fn set_timeout(&mut self, timeout: std::time::Duration) {
        self.transport.set_timeout(timeout);
    }
}

impl<S: may_rpc::StreamExt> HelloClient<S> {
    pub fn echo(&self, data: String) -> Result<String, may_rpc::Error> {
        use may_rpc::Client;

        let mut req = may_rpc::ReqBuf::new();
        // serialize the request
        let request = HelloRequest::Echo { data };
        req.write_all(&bitcode::encode(&request)).unwrap();
        // call the server
        let rsp_frame = self.transport.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        bitcode::decode(rsp).map_err(|e| may_rpc::Error::ClientDeserialize(e.to_string()))
    }

    pub fn add(&self, x: u32, y: u32) -> Result<u32, may_rpc::Error> {
        use may_rpc::Client;

        let mut req = may_rpc::ReqBuf::new();
        // serialize the request
        let request = HelloRequest::Add { x, y };
        req.write_all(&bitcode::encode(&request)).unwrap();
        // call the server
        let rsp_frame = self.transport.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        bitcode::decode(rsp).map_err(|e| may_rpc::Error::ClientDeserialize(e.to_string()))
    }
}

pub trait HelloServiceDispatch: Hello + std::panic::RefUnwindSafe {
    fn dispatch_req(
        &self,
        request: HelloRequest,
        rsp: &mut may_rpc::RspBuf,
    ) -> Result<(), may_rpc::WireError> {
        // dispatch call the service
        match request {
            HelloRequest::Echo { data } => match std::panic::catch_unwind(|| self.echo(data)) {
                Ok(ret) => rsp
                    .write_all(&bitcode::encode(&ret))
                    .map_err(|e| may_rpc::WireError::ServerSerialize(e.to_string())),
                Err(_) => Err(may_rpc::WireError::Status(
                    "rpc panicked in server!".to_owned(),
                )),
            },
            HelloRequest::Add { x, y } => match std::panic::catch_unwind(|| self.add(x, y)) {
                Ok(ret) => rsp
                    .write_all(&bitcode::encode(&ret))
                    .map_err(|e| may_rpc::WireError::ServerSerialize(e.to_string())),
                Err(_) => Err(may_rpc::WireError::Status(
                    "rpc panicked in server!".to_owned(),
                )),
            },
        }
    }
}

impl<T: Hello + std::panic::RefUnwindSafe> HelloServiceDispatch for T {}

mod server {
    use super::{Hello, HelloRequest};
    pub struct HelloService;

    impl Hello for HelloService {
        fn echo(&self, data: String) -> String {
            data
        }

        fn add(&self, x: u32, y: u32) -> u32 {
            x + y
        }
    }

    impl may_rpc::Server for HelloService {
        fn service(&self, req: &[u8], rsp: &mut may_rpc::RspBuf) -> Result<(), may_rpc::WireError> {
            use super::HelloServiceDispatch;

            // deserialize the request
            let request: HelloRequest = bitcode::decode(req)
                .map_err(|e| may_rpc::WireError::ServerDeserialize(e.to_string()))?;

            log::info!("request = {:?}", request);

            // get the dispatch_fn
            self.dispatch_req(request, rsp)
        }
    }
}

use std::io::Write;

pub use server::HelloService;
