use std::fmt::Display;

#[derive(Debug)]
pub enum DispatchError {
    Transient(String),
    Permanent(String),
}

impl Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::Transient(msg) => write!(f, "transient: {}", msg),
            DispatchError::Permanent(msg) => write!(f, "permanent: {}", msg),
        }
    }
}

impl std::error::Error for DispatchError {}
