use futures::{
    future::join_all,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use log::*;
use shared_types::messages::{ClientMessage, MessageContents};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::{
    tungstenite::{
        handshake::server::{ErrorResponse, Request, Response},
        Message,
    },
    WebSocketStream,
};

use crate::{config::ServerConfig, error::ServerError};

struct User {
    name: String,
    writable_message_sink: SplitSink<WebSocketStream<TcpStream>, Message>,
}

type Users = Arc<Mutex<HashMap<SocketAddr, User>>>;

pub(crate) struct Server {
    connected_users: Users,
    config: ServerConfig,
}

impl Server {
    pub(crate) fn new(config: ServerConfig) -> Self {
        Self {
            connected_users: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    async fn accept_connection(
        mut stream: SplitStream<WebSocketStream<TcpStream>>,
        client_socket_addr: SocketAddr,
        connected_users: Users,
    ) {
        //TODO: Impl crypto https://docs.rs/simple_crypt/latest/simple_crypt/
        debug!("Polling {client_socket_addr} for messages");
        while let Some(Ok(message)) = stream.next().await {
            debug!("New message: {message} \n\t from {client_socket_addr}, propogating to connected clients");
            let mut connected_users_lock = connected_users.lock().await;
            let client_name = connected_users_lock
                .get(&client_socket_addr)
                .expect("Should not return None as map must be synced with active ws connections")
                .name
                .clone();
            let try_serialized_message = match message {
                Message::Text(text_message) => {
                    let client_message = ClientMessage {
                        author: client_name,
                        contents: MessageContents::Text(text_message),
                    };
                    let serialized_message = serde_json::to_string(&client_message);
                    serialized_message
                }
                Message::Binary(file) => {
                    if let Some(Ok(Message::Text(file_name))) = stream.next().await {
                        let client_message = ClientMessage {
                            author: client_name,
                            contents: MessageContents::File {
                                name: String::from("test"),
                                contents: file,
                            },
                        };

                        let serialized_message = serde_json::to_string(&client_message);
                        serialized_message
                    } else {
                        //TODO: Add error type to match serde and the situation
                        //      where no file name is sent and make this varia-
                        //      ble use the library error type
                        todo!()
                    }
                }
                Message::Close(_) => {
                    let client_message = ClientMessage {
                        author: String::from("Server"),
                        contents: MessageContents::Text(format!("{client_name} has disconnected")),
                    };
                    let serialized_message = serde_json::to_string(&client_message);

                    connected_users_lock.remove(&client_socket_addr);

                    serialized_message
                }
                _ => todo!(),
            };
            match try_serialized_message {
                Ok(serialized_message) => {
                    let message_to_propogate = Message::text(serialized_message);

                    let mut futures_batched = vec![];
                    // Send message to everyone else but the user that sent it
                    connected_users_lock
                        .iter_mut()
                        .filter(|(addr, _)| **addr != client_socket_addr)
                        .for_each(|(_, user)| {
                            futures_batched.push(
                                user.writable_message_sink
                                    .send(message_to_propogate.clone()),
                            );
                        });
                    join_all(futures_batched).await;
                }
                Err(err) => {
                    //TODO: Send message back to this client and say that we could not process the message
                    todo!()
                }
            }
        }
    }

    pub(crate) async fn run_server(self) -> Result<(), ServerError> {
        info!("Starting the server");

        let config = self.config;
        let server_socket_addr = SocketAddr::new(config.ip_addr, config.port);
        debug!("Trying to use socket address: {server_socket_addr}");

        let listener = TcpListener::bind(server_socket_addr)
            .await
            .map_err(|error| {
                error!("Error binding to socket address: {error:?}");
                ServerError::TCPBindError(error)
            })?;
        debug!("TCP server listening on: {server_socket_addr}");

        while let Ok((stream, client_socket_addr)) = listener.accept().await {
            let mut username = String::from("");
            let try_ws_stream =
                tokio_tungstenite::accept_hdr_async(stream, |req: &Request, response: Response| {
                    debug!("Received a new ws handshake");
                    debug!("The request's path is: {}", req.uri().path());
                    debug!("The request's headers are:");
                    for (ref header, value) in req.headers() {
                        debug!("* {}: {:?}", header, value);
                        // Can name yourself with anything but any variations of 'server'
                        // will be reserved user to send announcements
                        if let Ok(string_value) = value.to_str() {
                            if *header == "username" && string_value.to_lowercase() != "server" {
                                username.push_str(string_value);
                            }
                        }
                    }
                    if username.is_empty() {
                        let err_response = ErrorResponse::new(Some(String::from(
                            "Did not provide valid username",
                        )));
                        error!("Did not provide valid username");
                        Err(err_response)
                    } else {
                        Ok(response)
                    }
                })
                .await
                .map_err(|error| {
                    error!("Error binding to socket address: {error:?}");
                    ServerError::CreateWebsocketError(error)
                });

            match try_ws_stream {
                Ok(mut ws_stream) => {
                    info!("New websocket connection from: {client_socket_addr}");
                    let mut connected_users_lock = self.connected_users.lock().await;

                    if connected_users_lock.contains_key(&client_socket_addr) {
                        info!("New websocket connection denied: {client_socket_addr}");
                        debug!("User is already connected from this IP");
                        //TODO: Send announcemnt from the server
                        ws_stream.close(None).await;
                    } else if config.banned_users.contains(&client_socket_addr.ip()) {
                        info!("New websocket connection denied: {client_socket_addr}");
                        debug!("User is banned");
                        //TODO: Send announcemnt from the server
                        ws_stream.close(None).await;
                    } else {
                        // Splitting into read and write portions of the connections,
                        // move the readable to the spawned handler as it is not needed for
                        // anything else, while the writeable to the map of users
                        let (sink, stream) = ws_stream.split();

                        let new_user = User {
                            name: username,
                            writable_message_sink: sink,
                        };
                        connected_users_lock.insert(client_socket_addr, new_user);
                        // Early drop since it is not used anywhere else after
                        drop(connected_users_lock);

                        let handler = Self::accept_connection(
                            stream,
                            client_socket_addr,
                            self.connected_users.clone(),
                        );
                        // TODO: Add some sort of 'annoucement' to other users that
                        // somebody joined
                        info!("User handshake complete for: {client_socket_addr}");
                        tokio::spawn(handler);
                    }
                }
                Err(_) => continue, // Don't really care why the handshake failed
            }
        }

        Ok(())
    }
}
