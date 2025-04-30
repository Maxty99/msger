//TODO: CHange dyn to generics and make return type of connect use <impl Sink/Steam> and add another trait for convenience methods

use std::pin::Pin;

use base64::{Engine, prelude::BASE64_STANDARD};
use futures::{Sink, SinkExt, Stream, StreamExt, stream::FusedStream};
use shared_types::messages::ClientMessage;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{
        Message, client::IntoClientRequest, handshake::client::Request, http::HeaderValue,
    },
};

use crate::error::ClientError;

pub type WSMessage = Message;

pub async fn connect(
    username: &str,
    maybe_password: Option<String>,
    server_address: &str,
) -> Result<ChatSession, ClientError> {
    let mut req = server_address
        .into_client_request()
        .map_err(|err| ClientError::CreateWSConnectionError(err))?;

    req.headers_mut()
        .append("username", HeaderValue::from_str(username)?);

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
        maybe_password,
    })
}

pub struct ChatSession {
    inner: WebSocketStream<MaybeTlsStream<TcpStream>>,
    maybe_password: Option<String>,
}

impl Stream for ChatSession {
    type Item = Result<ClientMessage, ClientError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner
            .poll_next_unpin(cx)
            .map(|ws_message| match ws_message {
                None => None,
                Some(Ok(Message::Text(message_string))) => {
                    let try_parsed_message = serde_json::from_str::<ClientMessage>(&message_string);
                    match try_parsed_message {
                        Ok(parsed_message) => {
                            // Decrypt id password specified
                            if let Some(ref password) = self.maybe_password {
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

impl Sink<WSMessage> for ChatSession {
    type Error = ClientError;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready_unpin(cx)
            .map_err(ClientError::SendMessageError)
    }

    fn start_send(mut self: Pin<&mut Self>, item: WSMessage) -> Result<(), Self::Error> {
        self.inner
            .start_send_unpin(item)
            .map_err(ClientError::SendMessageError)
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

impl ChatSession {
    pub async fn send_message<T: ToString + Send>(
        &mut self,
        message: T,
    ) -> Result<(), ClientError> {
        self.send(Message::Text(message.to_string())).await
    }

    pub async fn send_file<T: Into<Vec<u8>> + Send>(&mut self, file: T) -> Result<(), ClientError> {
        self.send(Message::Binary(file.into())).await
    }

    pub async fn disconnect(&mut self) -> Result<(), ClientError> {
        self.send(Message::Close(None)).await
    }
}
