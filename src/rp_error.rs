use std::fmt;
use std::error;

// The "no file or yaml error type"
#[derive(Debug)]
pub enum RPError {
    IOError(std::io::Error),
    YAMLError(serde_yaml::Error),
}
impl fmt::Display for RPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RPError::IOError(ref e) => e.fmt(f),
            // This is a wrapper, so defer to the underlying types' implementation of `fmt`.
            RPError::YAMLError(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for RPError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            RPError::IOError(ref e) => Some(e),
            // The cause is the underlying implementation error type. Is implicitly
            // cast to the trait object `&error::Error`. This works because the
            // underlying type already implements the `Error` trait.
            RPError::YAMLError(ref e) => Some(e),
        }
    }
}

// Implement the conversion from `serde_yaml::Error` to `RPError`.
// This will be automatically called by `?` if a `serde_yaml::Error`
// needs to be converted into a `RPError`.
impl From<serde_yaml::Error> for RPError {
    fn from(err: serde_yaml::Error) -> RPError {
        RPError::YAMLError(err)
    }
}
impl From<std::io::Error> for RPError {
    fn from(err: std::io::Error) -> RPError {
        RPError::IOError(err)
    }
}