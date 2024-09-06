use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Could not create a WS connection to the server: {0}")]
    CreateWSConnectionError(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not create the request needed to establish the WS connection")]
    CreateWSRequestError(#[from] tokio_tungstenite::tungstenite::http::Error),

    #[error("Could not receive message from server")]
    ReceiveIncomingMessageError(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not interpret message from server")]
    ParseIncomingMessageError(#[from] serde_json::error::Error),

    #[error("Message from server was not the expected format")]
    IncomingMessageFormatError,

    #[error("Could not send message to server")]
    SendMessageError(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Could not form request to connect since the username is invalid")]
    UsernameError(#[from] tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue),
}
