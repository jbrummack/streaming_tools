use std::panic::Location;

pub mod lines;
pub mod progress;
pub mod proto;
pub mod streamer;
pub trait StreamingResultHandlerExt<T, E> {
    fn handle_eof(self, n_lines: usize) -> Result<Option<T>, E>;
}
impl<T> StreamingResultHandlerExt<T, std::io::Error> for Result<T, std::io::Error> {
    #[track_caller]
    fn handle_eof(self, n_lines: usize) -> Result<Option<T>, std::io::Error> {
        match self {
            Ok(success) => Ok(Some(success)),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => {
                let location = Location::caller();
                let what_happened = format!(
                    "[{location}] Unable to read line because of {e}, decoded {n_lines} Elements so far",
                );
                tracing::error!("{what_happened}");
                Err(std::io::Error::new(e.kind(), what_happened))
            }
        }
    }
}
