use std::io;

macro_rules! io_error {
    ($tag:expr) => {{
        |e| {
            let message = format!("{}: {:?}", $tag, e);
            io::Error::new(io::ErrorKind::Other, message)
        }
    }}
}
