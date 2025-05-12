use client::ChatWrite;
use futures::TryFutureExt;
use iced::futures::Stream;
use iced::futures::channel::mpsc;
use iced::futures::sink::SinkExt;
use iced::futures::stream::StreamExt;
use iced::stream::channel;

use client::ClientMessage;
use shared_types::messages::ServerMessage;

use crate::{AppUpdateMessage, ErrorPopupMessage};

use super::ChatPageMessage;

/// Shorthand type for the mpsc Sender responsible for communicating with the worker
pub(super) type ChatSender = mpsc::Sender<ClientMessage>;

/// Start the background chat worker to handle sending messages to the
/// WebSocket server. Need to pass username that the user chose during
/// the connection step.
pub(super) fn start_chat_worker(
    mut chat_session_writer: impl ChatWrite,
    username: String,
) -> impl Stream<Item = AppUpdateMessage> {
    channel(100, async move |mut output| {
        // Create channel
        let (sender, mut receiver) = mpsc::channel::<ClientMessage>(100);

        // Send the sender back to the application
        output
            .send(ChatPageMessage::WorkerReady(sender).into())
            .await
            .expect(
                "chat worker should be able to return the sender back to the chat_page component",
            ); // If gives error its probably not recoverable

        let mut disconnected = false;

        // Read next available command in the form of the desired
        // ClientMessage to be sent:
        while let Some(chat_message_to_send) = receiver.next().await {
            if disconnected {
                break;
            }
            // Perform appropriate client action:
            let chat_message_send_result = match chat_message_to_send {
                // If all went well we want to add the message to the history.
                // We make a local ServerMessage to keep it simple and treat
                // all messages the same and then pass it on as if it was
                // received from the server
                ClientMessage::Text(text_msg) => {
                    let server_message = ServerMessage::text(&username, &text_msg);
                    chat_session_writer
                        .send_message(text_msg)
                        .and_then(async |_| Ok(server_message))
                        .await
                }
                ClientMessage::File(filename, file_contents) => {
                    let server_message =
                        ServerMessage::file(&username, &filename, file_contents.as_slice());
                    chat_session_writer
                        .send_file(filename, file_contents)
                        .and_then(async |_| Ok(server_message))
                        .await
                }
                ClientMessage::Disconnect => {
                    // Assume disconnected even if it returns an error
                    // server should handle idle users in the case it
                    // doesn't receive the disconnect message
                    disconnected = true;
                    chat_session_writer
                        .disconnect()
                        .and_then(async |_| Ok(ServerMessage::disconnect_message()))
                        .await
                }
            };
            // Map result of client action to a message to update the GUI:
            match chat_message_send_result {
                Ok(sent_chat_message) => {
                    output
                        .send(ChatPageMessage::AddMessageToHistory(sent_chat_message).into())
                        .await
                }
                Err(err) => {
                    output
                        .send(ErrorPopupMessage::AddError(err.to_string()).into())
                        .await
                }
            }
            // If we get an Err variant its probably unrecoverable
            .expect("receiver should not drop while we are in this loop");
        }
        // TODO: Figure out why it doesn't reach this part of the code on disconnect
        // Ensure final message notifies app to discard other side of channel
        // If it fails then the page will be stuck on the chat softlocking the app
        output
            .send(ChatPageMessage::ResetChatWorker.into())
            .await
            .expect("receiver should not drop untill after this call");
    })
}
