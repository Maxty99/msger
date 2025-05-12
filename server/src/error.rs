use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Could not read config file from filesystem (Different from parsing error)")]
    ReadConfig(#[source] std::io::Error),

    #[error("Could not parse config file (Different from read error)")]
    ParseConfig(#[from] toml::de::Error),

    #[error("Could not bind to provided address")]
    TCPBind(#[source] std::io::Error),

    #[error("Could not build a websocket connection")]
    CreateWebsocket(#[source] tokio_tungstenite::tungstenite::Error),
}
