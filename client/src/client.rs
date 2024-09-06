//TODO: CHange dyn to generics and make return type of connect use <impl Sink/Steam> and add another trait for convenience methods

use std::pin::Pin;

use futures::{stream::FusedStream, Sink, SinkExt, Stream, StreamExt};
use shared_types::messages::ClientMessage;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest, handshake::client::Request, http::HeaderValue, Message,
    },
    MaybeTlsStream, WebSocketStream,
};

use crate::error::ClientError;

pub type WSMessage = Message;

pub async fn connect(username: &str, server_address: &str) -> Result<Client, ClientError> {
    let mut req = server_address
        .into_client_request()
        .map_err(|err| ClientError::CreateWSConnectionError(err))?;

    req.headers_mut()
        .append("username", HeaderValue::from_str(username)?);


    let (ws_stream, _resp) = connect_async(req)
        .await
        .map_err(|err| ClientError::CreateWSConnectionError(err))?;

    Ok(Client { inner: ws_stream })
}

pub struct Client {
    inner: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Stream for Client {
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
                        Ok(parsed_message) => Some(Ok(parsed_message)),
                        Err(err) => Some(Err(ClientError::ParseIncomingMessageError(err))),
                    }
                }
                Some(Err(err)) => Some(Err(ClientError::ReceiveIncomingMessageError(err))),
                // Catches all messages that are not text
                _ => Some(Err(ClientError::IncomingMessageFormatError)),
            })
    }
}

impl FusedStream for Client {
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }
}

impl Sink<WSMessage> for Client {
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

impl Client {
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
