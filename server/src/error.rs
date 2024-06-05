use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Could not read config file from filesystem (Different from parsing error)")]
    ReadConfigError(#[from] std::io::Error),

    #[error("Could not parse config file (Different from read error)")]
    ParseConfigError(#[from] toml::de::Error),
}
