#[derive(Debug)]
pub enum StateError {
    Outdated,
    InvalidHeader,
    InvalidOffset,
    InvalidData(String),
    InvalidHash,
    Io(std::io::Error),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Outdated => f.write_str("Outdated file"),
            Self::InvalidHeader => f.write_str("Invalid undofile header"),
            Self::InvalidOffset => f.write_str("Invalid merge offset"),
            Self::InvalidData(msg) => f.write_str(msg),
            Self::InvalidHash => f.write_str("invalid hash for undofile itself"),
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for StateError {}

impl From<std::io::Error> for StateError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
