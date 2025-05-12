use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Could not create a WS connection to the server: {0}")]
    CreateWSConnection(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not create the request needed to establish the WS connection")]
    CreateWSRequest(#[from] tokio_tungstenite::tungstenite::http::Error),

    #[error("Could not receive message from server")]
    ReceiveIncomingMessage(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not interpret message from server")]
    ParseIncomingMessage(#[from] serde_json::error::Error),

    #[error("Message from server was not the expected format")]
    IncomingMessageFormat,

    #[error("Could not send message to server")]
    SendMessage(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not properly send disconnect message to server")]
    SendDisconnect(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not form request to connect since the username is invalid")]
    BadUsername(#[from] tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue),

    #[error("Server sent beck malformed password test string")]
    PasswordErrorBase64(#[from] base64::DecodeError),

    #[error("Password decrypted test string did not match expected string")]
    BadPassword,
}
