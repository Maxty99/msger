//TODO: CHange dyn to generics and make return type of connect use <impl Sink/Steam> and add another trait for convenience methods

use std::pin::Pin;

use base64::{Engine, prelude::BASE64_STANDARD};
use futures::{
    Sink, SinkExt, Stream, StreamExt,
    stream::{FusedStream, SplitSink, SplitStream},
};
use shared_types::messages::ServerMessage;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message as WSMessage, client::IntoClientRequest, http::HeaderValue},
};

use crate::error::ClientError;

pub async fn connect(
    username: String,
    maybe_password: Option<String>,
    server_address: String,
) -> Result<ChatSession, ClientError> {
    let mut req = server_address
        .into_client_request()
        .map_err(|err| ClientError::CreateWSConnectionError(err))?;

    req.headers_mut()
        .append("username", HeaderValue::from_str(&username)?);

    let (ws_stream, resp) = connect_async(req)
        .await
        .map_err(|err| ClientError::CreateWSConnectionError(err))?;

    for (ref header, value) in resp.headers() {
        if let Ok(string_value) = value.to_str() {
            if *header == shared_types::crypt::CRYPT_VALIDATION_KEY {
                // Decrypt and compare value
                // First layer: Base64
                let encrypted_test_value = BASE64_STANDARD.decode(string_value)?;
                // Second layer: simple_crypt (Contingent on Some(password))
                let test_value = if let Some(ref password) = maybe_password {
                    simple_crypt::decrypt(encrypted_test_value.as_slice(), password.as_bytes())
                        .map_err(|_| ClientError::PasswordError)?
                } else {
                    // If there isnt then just compare base64 decoded val
                    encrypted_test_value
                };
                if String::from_utf8_lossy(&test_value) != shared_types::crypt::CRYPT_VALIDATION_VAL
                {
                    return Err(ClientError::PasswordError);
                }
            }
        }
    }

    Ok(ChatSession {
        inner: ws_stream,
        password: maybe_password.map(|some_pass| String::from(some_pass)),
    })
}

/// Struct that controls a sinlge chat session
#[derive(Debug)]
pub struct ChatSession {
    inner: WebSocketStream<MaybeTlsStream<TcpStream>>,
    password: Option<String>,
}

/// The allowed types of messages that can be sent to the server
#[derive(Debug)]
pub enum ClientMessage {
    Text(String),
    File(String, Vec<u8>),
    // Treating disconnecting as a pseudo-message simplifies some logic
    Disconnect,
}

impl ClientMessage {
    #[inline(always)]
    pub const fn disconnect_message() -> Self {
        ClientMessage::Disconnect
    }

    pub fn text<S: ToString>(msg: S) -> Self {
        Self::Text(msg.to_string())
    }

    pub fn file<S: ToString, B: Into<Vec<u8>>>(filename: S, file_as_bytes: B) -> Self {
        Self::File(filename.to_string(), file_as_bytes.into())
    }
}

impl Stream for ChatSession {
    type Item = Result<ServerMessage, ClientError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner
            .poll_next_unpin(cx)
            .map(|ws_message| match ws_message {
                None => None,
                Some(Ok(WSMessage::Text(message_string))) => {
                    let try_parsed_message = serde_json::from_str::<ServerMessage>(&message_string);
                    match try_parsed_message {
                        Ok(parsed_message) => {
                            // Decrypt id password specified
                            if let Some(ref password) = self.password {
                                todo!()
                            } else {
                                Some(Ok(parsed_message))
                            }
                        }
                        Err(err) => Some(Err(ClientError::ParseIncomingMessageError(err))),
                    }
                }
                Some(Err(err)) => Some(Err(ClientError::ReceiveIncomingMessageError(err))),
                // Catches all messages that are not text
                _ => Some(Err(ClientError::IncomingMessageFormatError)),
            })
    }
}

impl FusedStream for ChatSession {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

impl Sink<ClientMessage> for ChatSession {
    type Error = ClientError;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready_unpin(cx)
            .map_err(ClientError::SendMessageError)
    }

    fn start_send(mut self: Pin<&mut Self>, item: ClientMessage) -> Result<(), Self::Error> {
        match item {
            ClientMessage::Text(msg) => {
                let converted_to_ws_message = WSMessage::text(msg);
                self.inner
                    .start_send_unpin(converted_to_ws_message)
                    .map_err(ClientError::SendMessageError)
            }
            ClientMessage::File(filename, file_as_bytes) => {
                let filename_to_ws_message = WSMessage::text(filename);
                let file_to_ws_message = WSMessage::binary(file_as_bytes);
                // Server expects filename to be sent immediately after the
                // binary message
                self.inner
                    .start_send_unpin(file_to_ws_message)
                    .and_then(|_| self.inner.start_send_unpin(filename_to_ws_message))
                    .map_err(ClientError::SendMessageError)
            }
            ClientMessage::Disconnect => self
                .inner
                .start_send_unpin(WSMessage::Close(None))
                .map_err(ClientError::SendDisconnectError),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_flush_unpin(cx)
            .map_err(ClientError::SendMessageError)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_close_unpin(cx)
            .map_err(ClientError::SendMessageError)
    }
}

/// Convenience type for the [Stream] (read) side of the split [ChatSession]
pub type ChatSessionReader = SplitStream<ChatSession>;

/// Convenience type for the [Sink] (Write) side of the split [ChatSession]
pub type ChatSessionWriter = SplitSink<ChatSession, ClientMessage>;

pub trait ChatWrite {
    fn send_message<T: ToString + Send>(
        &mut self,
        message: T,
    ) -> impl Future<Output = Result<(), ClientError>> + Send;
    fn send_file<S: ToString + Send, B: Into<Vec<u8>> + Send>(
        &mut self,
        file_name: S,
        file_as_bytes: B,
    ) -> impl Future<Output = Result<(), ClientError>> + Send;
    fn disconnect(&mut self) -> impl Future<Output = Result<(), ClientError>> + Send;
}

impl ChatWrite for ChatSessionWriter {
    async fn send_message<T: ToString + Send>(&mut self, message: T) -> Result<(), ClientError> {
        self.send(ClientMessage::Text(message.to_string())).await
    }

    async fn send_file<S: ToString + Send, B: Into<Vec<u8>> + Send>(
        &mut self,
        file_name: S,
        file_as_bytes: B,
    ) -> Result<(), ClientError> {
        self.send(ClientMessage::File(
            file_name.to_string(),
            file_as_bytes.into(),
        ))
        .await
    }

    async fn disconnect(&mut self) -> Result<(), ClientError> {
        self.send(ClientMessage::Disconnect).await
    }
}

impl ChatWrite for ChatSession {
    async fn send_message<T: ToString + Send>(&mut self, message: T) -> Result<(), ClientError> {
        self.send(ClientMessage::Text(message.to_string())).await
    }

    async fn send_file<S: ToString + Send, B: Into<Vec<u8>> + Send>(
        &mut self,
        file_name: S,
        file_as_bytes: B,
    ) -> Result<(), ClientError> {
        self.send(ClientMessage::File(
            file_name.to_string(),
            file_as_bytes.into(),
        ))
        .await
    }

    async fn disconnect(&mut self) -> Result<(), ClientError> {
        self.send(ClientMessage::Disconnect).await
    }
}
