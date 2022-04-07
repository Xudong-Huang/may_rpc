pub trait Hello {
    fn echo(&self, data: String) -> String;
    fn add(&self, x: u32, y: u32) -> u32;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum HelloRequest {
    Echo { data: String },
    Add { x: u32, y: u32 },
}

#[derive(Debug)]
pub struct HelloClient<S: conetty::StreamExt> {
    transport: conetty::MultiplexClient<S>,
}

impl<S: conetty::StreamExt> HelloClient<S> {
    pub fn new(stream: S) -> std::io::Result<Self> {
        let transport = conetty::MultiplexClient::new(stream)?;
        Ok(Self { transport })
    }
    pub fn set_timeout(&mut self, timeout: std::time::Duration) {
        self.transport.set_timeout(timeout);
    }
}

impl<S: conetty::StreamExt> HelloClient<S> {
    pub fn echo(&self, data: String) -> Result<String, conetty::Error> {
        use conetty::Client;

        let mut req = conetty::ReqBuf::new();
        // serialize the request
        let request = HelloRequest::Echo { data };
        bincode::serialize_into(&mut req, &request)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.transport.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        bincode::deserialize(rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }

    pub fn add(&self, x: u32, y: u32) -> Result<u32, conetty::Error> {
        use conetty::Client;

        let mut req = conetty::ReqBuf::new();
        // serialize the request
        let request = HelloRequest::Add { x, y };
        bincode::serialize_into(&mut req, &request)
            .map_err(|e| conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.transport.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        bincode::deserialize(rsp).map_err(|e| conetty::Error::ClientDeserialize(e.to_string()))
    }
}

pub trait HelloServiceDispatch: Hello + std::panic::RefUnwindSafe {
    fn dispatch_req(
        &self,
        request: HelloRequest,
        rsp: &mut conetty::RspBuf,
    ) -> Result<(), conetty::WireError> {
        // dispatch call the service
        match request {
            HelloRequest::Echo { data } => match std::panic::catch_unwind(|| self.echo(data)) {
                Ok(ret) => bincode::serialize_into(rsp, &ret)
                    .map_err(|e| conetty::WireError::ServerSerialize(e.to_string())),
                Err(_) => Err(conetty::WireError::Status(
                    "rpc panicked in server!".to_owned(),
                )),
            },
            HelloRequest::Add { x, y } => match std::panic::catch_unwind(|| self.add(x, y)) {
                Ok(ret) => bincode::serialize_into(rsp, &ret)
                    .map_err(|e| conetty::WireError::ServerSerialize(e.to_string())),
                Err(_) => Err(conetty::WireError::Status(
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

    impl conetty::Server for HelloService {
        fn service(&self, req: &[u8], rsp: &mut conetty::RspBuf) -> Result<(), conetty::WireError> {
            use super::HelloServiceDispatch;

            // deserialize the request
            let request: HelloRequest = bincode::deserialize(req)
                .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;

            log::info!("request = {:?}", request);

            // get the dispatch_fn
            self.dispatch_req(request, rsp)
        }
    }
}

pub use server::HelloService;
