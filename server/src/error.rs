
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Could not read config file from filesystem (Different from parsing error)")]
    ReadConfigError(#[source] std::io::Error),

    #[error("Could not parse config file (Different from read error)")]
    ParseConfigError(#[from] toml::de::Error),

    #[error("Could not bind to provided address")]
    TCPBindError(#[source] std::io::Error),

    #[error("Could not build a websocket connection")]
    CreateWebsocketError(#[source] tokio_tungstenite::tungstenite::Error),
}
