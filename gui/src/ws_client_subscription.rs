use client::client::{connect, Client, WSMessage};
use iced::subscription::channel;
use iced::{futures, Subscription};

use futures::channel::mpsc::{self, Receiver};
use futures::sink::SinkExt;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use shared_types::messages::ClientMessage;

use crate::AppUpdateMessage;

pub fn start_client() -> Subscription<AppUpdateMessage> {
    struct ClientBackgroundTask; // Type to keep track of unique task
    let mut worker_state = WorkerState::Setup;
    let mut client_state = ConnectionState::Idle;
    channel(
        std::any::TypeId::of::<ClientBackgroundTask>(),
        100,
        |mut output| async move {
            loop {
                match worker_state {
                    WorkerState::Setup => {
                        // Create channel
                        let (sender, receiver) = mpsc::channel(100);

                        let _ = output.send(AppUpdateMessage::AppReady(sender)).await;
                        worker_state = WorkerState::Ready(receiver);
                    }
                    WorkerState::Ready(ref mut receiver) => match client_state {
                        ConnectionState::Idle => {
                            let input = receiver.select_next_some().await;
                            match input {
                                ClientCommand::Connect(username, server_address) => {
                                    let try_client = connect(&username, &server_address).await;
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
                            futures::select! {
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
        },
    )
}

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
