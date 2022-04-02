pub trait Hello {
    fn echo(data: String) -> String;
    fn add(x: u32, y: u32) -> u32;
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

impl<S: crate::conetty::StreamExt> HelloClient<S> {
    pub fn echo(&self, data: String) -> Result<String, crate::conetty::Error> {
        use crate::conetty::Client;

        let mut req = crate::conetty::ReqBuf::new();
        // serialize the request
        let request = HelloRequest::Echo { data };
        bincode::serialize_into(&mut req, &request)
            .map_err(|e| crate::conetty::Error::ClientSerialize(e.to_string()))?;
        // call the server
        let rsp_frame = self.transport.call_service(req)?;
        let rsp = rsp_frame.decode_rsp()?;
        // deserialized the response
        bincode::deserialize(rsp).map_err(|e| crate::conetty::Error::ClientDeserialize(e.to_string()))
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

pub trait HelloServiceDispatch: Hello {
    fn dispatch_req(
        request: HelloRequest,
        rsp: &mut conetty::RspBuf,
    ) -> Result<(), conetty::WireError> {
        // dispatch call the service
        match request {
            HelloRequest::Echo { data } => bincode::serialize_into(rsp, &Self::echo(data))
                .map_err(|e| conetty::WireError::ServerSerialize(e.to_string())),
            HelloRequest::Add { x, y } => bincode::serialize_into(rsp, &Self::add(x, y))
                .map_err(|e| conetty::WireError::ServerSerialize(e.to_string())),
        }
    }
}

impl<T: Hello> HelloServiceDispatch for T {}

mod server {
    use super::{Hello, HelloRequest, HelloServiceDispatch};
    pub struct HelloService;

    impl Hello for HelloService {
        fn echo(data: String) -> String {
            data
        }

        fn add(x: u32, y: u32) -> u32 {
            x + y
        }
    }

    impl conetty::Server for HelloService {
        fn service(&self, req: &[u8], rsp: &mut conetty::RspBuf) -> Result<(), conetty::WireError> {
            use bincode as encode;

            // deserialize the request
            let request: HelloRequest = encode::deserialize(req)
                .map_err(|e| conetty::WireError::ServerDeserialize(e.to_string()))?;

            log::info!("request = {:?}", request);

            // get the dispatch_fn
            Self::dispatch_req(request, rsp)
        }
    }
}

pub use server::HelloService;
