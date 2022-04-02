/// define the Echo interface
/// this would also generate a client implementation
#[may_rpc::service]
pub trait Hello {
    /// Returns a greeting for name.
    fn echo(data: String) -> String;
	/// add two u32
	fn add(x: u32, y: u32) -> u32;
}

#[may_rpc::server]
pub struct HelloService;

/// implement the server
impl Hello for HelloService {
	fn echo(data: String) -> String {
		data
	}

	fn add(x: u32, y: u32) -> u32 {
		x + y
	}
}
