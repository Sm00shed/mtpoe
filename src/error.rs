use thiserror::Error;

#[derive(Debug, Error)]
pub enum MtpoeError {
    #[error("SPI error: {0}")]
    Spi(String),

    #[error("SPI CRC error: {0}")]
    SpiCrc(String),

    #[error("SPI command mismatch: {0}")]
    SpiCmd(String),

    #[error("board detection failed: {0}")]
    BoardDetection(String),

    #[error("UCI error: {0}")]
    Uci(String),

    #[error("invalid port: {0}")]
    InvalidPort(String),

    #[error("invalid value: {0}")]
    InvalidValue(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
