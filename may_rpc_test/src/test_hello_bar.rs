/// define the Echo interface
/// this would also generate a client implementation
#[may_rpc::service]
pub trait Hello {
    /// Returns a greeting for name.
    fn echo(&self, data: String) -> String;
    /// add two u32
    fn add(&self, x: u32, y: u32) -> u32;
}

#[derive(may_rpc::Server)]
#[service(Hello)]
pub struct HelloService;

/// implement the server
impl Hello for HelloService {
    fn echo(&self, data: String) -> String {
        data
    }

    fn add(&self, x: u32, y: u32) -> u32 {
        x + y
    }
}
