use client::client::{connect, Client};
use futures::Stream;
use iced::stream::channel;
use iced::{futures, Subscription};

use futures::channel::mpsc::{self, Receiver};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use shared_types::messages::ClientMessage;

use crate::AppUpdateMessage;

enum WorkerState {
    Setup,
    Ready(Receiver<ClientCommand>),
}

enum ConnectionState {
    Idle,
    Connected(Client),
}

pub enum ClientCommand {
    Connect(String, String),
    SendMessage(String),
    SendFile(Vec<u8>),
    Disconnect,
}

pub fn start_client() -> impl Stream<Item = AppUpdateMessage> {
    // Set up a background worker to keep track of the WS connection in the Client
    let mut worker_state = WorkerState::Setup; // Unique the the workier
    let mut client_state = ConnectionState::Idle; // Unique to the client
    channel(100, |mut output| async move {
        loop {
            match worker_state {
                WorkerState::Setup => {
                    //Basically we have no way to communicate with the frontend

                    let (sender, receiver) = mpsc::channel(100);

                    let _ = output.send(AppUpdateMessage::AppReady(sender)).await;

                    // Now we have something
                    worker_state = WorkerState::Ready(receiver);
                }
                WorkerState::Ready(ref mut receiver) => match client_state {
                    ConnectionState::Idle => {
                        let input = receiver.select_next_some().await;

                        match input {
                            ClientCommand::Connect(username, server_address) => {
                                let try_client = connect(&username, &server_address).await;

                                //Try to connect and abort if not possible
                                match try_client {
                                    Ok(client) => {
                                        let _ = output
                                            .send(AppUpdateMessage::UpdateAppView(
                                                crate::AppView::Chat,
                                            ))
                                            .await;

                                        client_state = ConnectionState::Connected(client);
                                    }
                                    Err(err) => {
                                        let _ = output
                                            .send(AppUpdateMessage::SetError(err.to_string()))
                                            .await;

                                        client_state = ConnectionState::Idle;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    ConnectionState::Connected(ref mut client) => {
                        // Can think of this as either getting a command from frontent or from server
                        futures::select! {

                        // The server wants us to do something
                        received = client.select_next_some() => {
                            match received {
                                Ok(message) => {
                                    let _ = output.send(AppUpdateMessage::MessageReceived(message)).await;
                                }
                                Err(err) => {
                                    let _ = output
                                    .send(AppUpdateMessage::SetError(err.to_string()))
                                    .await;

                                client_state = ConnectionState::Idle;
                                }
                            }
                        }

                        // The client wants us to do something
                        command = receiver.select_next_some() => {
                            match command {
                                ClientCommand::SendMessage(message) => {
                                        match client.send_message(&message).await {
                                            Ok(_) => {
                                                let _ = output
                                            .send(AppUpdateMessage::MessageReceived(ClientMessage {
                                                author: "You".to_string(),
                                                contents: shared_types::messages::MessageContents::Text(message)
                                            }))
                                            .await;
                                            },
                                            Err(err) => {
                                                let _ = output
                                                    .send(AppUpdateMessage::SetError(err.to_string()))
                                                    .await;

                                                client_state = ConnectionState::Idle;
                                            },
                                        }
                                    },
                                    ClientCommand::SendFile(file) => {
                                        match client.send_file(file.clone()).await {
                                            Ok(_) => {
                                                let _ = output
                                            .send(AppUpdateMessage::MessageReceived(ClientMessage {
                                                author: "You".to_string(),
                                                contents: shared_types::messages::MessageContents::File(file)
                                            }))
                                            .await;
                                            },
                                            Err(err) => {
                                                let _ = output
                                                    .send(AppUpdateMessage::SetError(err.to_string()))
                                                    .await;

                                                client_state = ConnectionState::Idle;
                                            },
                                        }
                                    },
                                    ClientCommand::Disconnect => {
                                        let _ = client.disconnect().await;
                                        let _ = output
                                                    .send(AppUpdateMessage::UpdateAppView(crate::AppView::Login))
                                                    .await;
                                        client_state = ConnectionState::Idle;
                                    },
                                    _ => continue,
                                }

                                }
                            }
                    }
                },
            }
        }
    })
}
