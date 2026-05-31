use std::panic::Location;
#[allow(unused)]
pub struct TracedError {
    inner: Error,
    location: &'static Location<'static>,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Server responded: {0}")]
    ServerStatus(&'static str),
    #[error("Client error: {0}")]
    ClientStatus(&'static str),
    #[error("HTTP Client: {0}")]
    Http(#[from] http::Error),
    #[error("HTTP Client: {0}")]
    Client(#[from] hyper_util::client::legacy::Error),
    #[error("Decode: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Encode: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Encode: {0}")]
    Io(#[from] std::io::Error),
}

impl std::fmt::Debug for TracedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TracedError")
            .field("inner", &self.inner)
            .field("location", &self.location)
            .finish()
    }
}
impl std::fmt::Display for TracedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]{}", self.location, self.inner)
    }
}
