use std::{fs::File, io::Read, sync::Arc};

use client::client::{ChatSessionWriter, ChatWrite};
use futures::TryFutureExt;
use iced::{
    Length, Task,
    widget::{button, column, row, scrollable, text, text_input},
};
use shared_types::messages::{MessageContents, ServerMessage};
use tokio::sync::Mutex;

use crate::AppUpdateMessage;

#[derive(Debug, Default)]
pub(crate) struct ChatPage {
    chat_messages: Vec<ServerMessage>,
    chat_input: String,
    chat_writer: Option<Arc<Mutex<ChatSessionWriter>>>,
}

#[derive(Debug, Clone)]
pub(crate) enum ChatPageMessage {
    NewChatWriter(Arc<Mutex<ChatSessionWriter>>),
    ResetChatWriter,
    UpdateChatInput(String),
    SendMessage(String),
    AddMessageToHistory(ServerMessage),
    AttemptSendFile,
    Disconnect,
}

impl ChatPage {
    pub(crate) fn view(&self) -> iced::Element<ChatPageMessage> {
        let messages = self.chat_messages.iter().fold(
            column![].align_x(iced::Alignment::Start).spacing(2),
            |col, message| {
                let message_author = text(format!("{}:", message.author));
                let message_contents = match &message.contents {
                    MessageContents::Text(txt) => text(txt),
                    // TODO: Add functionality to actually download file
                    MessageContents::File { name, contents } => text("This is a file wow"),
                };
                let message_row = row!(message_author, message_contents)
                    .spacing(2)
                    .align_y(iced::Alignment::End);
                col.push(message_row)
            },
        );
        let chat = scrollable(messages)
            .height(Length::FillPortion(9))
            .width(Length::Fill);

        let chat_input = text_input("Type your message...", &self.chat_input)
            .on_input(ChatPageMessage::UpdateChatInput)
            .on_submit(ChatPageMessage::SendMessage(self.chat_input.clone()))
            .padding(5);

        let chat_open_file_button = button(text("Open"))
            .on_press(ChatPageMessage::AttemptSendFile)
            .padding(5);

        let chat_submit_button = button(text("Send"))
            .on_press(ChatPageMessage::SendMessage(self.chat_input.clone()))
            .padding(5);
        let chat_disconnect_button = button(text("Disconnect"))
            .padding(5)
            .on_press(ChatPageMessage::Disconnect);
        let controls = row!(
            chat_input,
            chat_submit_button,
            chat_open_file_button,
            chat_disconnect_button
        )
        .height(Length::FillPortion(1));

        column!(chat, controls).into()
    }

    pub(crate) fn update(&mut self, message: ChatPageMessage) -> iced::Task<AppUpdateMessage> {
        match message {
            ChatPageMessage::SendMessage(msg) => {
                if let Some(ref mut writer_mutex) = self.chat_writer {
                    // Need to do this cloning to satisfy 'static bounds needed
                    let owned_writer_mutex = writer_mutex.clone(); // Cheap clone
                    let msg_clone = msg.clone(); // Probably cheap clone
                    let sent_chat_message = ServerMessage {
                        author: String::from("You"),
                        contents: MessageContents::Text(msg_clone),
                    };
                    return Task::perform(
                        async move {
                            let mut writer = owned_writer_mutex.lock().await;
                            writer.send_message(msg).map_ok(|_| sent_chat_message).await
                        },
                        move |disconnect_result| match disconnect_result {
                            Ok(sent_msg) => AppUpdateMessage::ChatPageMessage(
                                ChatPageMessage::AddMessageToHistory(sent_msg),
                            ),
                            Err(err) => AppUpdateMessage::AddError(err.to_string()),
                        },
                    );
                }
            }
            ChatPageMessage::AttemptSendFile => {
                // TODO: I wonder if I can do this better. Currently if you select a big a** file
                // you won't be able to store it in memory and probably lead to crash
                let maybe_file: Option<(String, Vec<u8>)> = rfd::FileDialog::new()
                    .pick_file()
                    // Make into (name, path) tuple
                    .and_then(|path_buf| {
                        Some((
                            // If not file indicates logic error with my code and RFD
                            // need something stricter than '?' operator so using expect
                            path_buf
                                .file_name()
                                .expect("file dialog to only give file")
                                .to_string_lossy() // Not big deal if name is a bit mangled
                                .into(),
                            File::open(path_buf).ok()?,
                        ))
                    })
                    // Make into (name, file contents) tuple
                    .and_then(|(filename, mut file)| {
                        let mut buf = vec![];
                        Some((filename, file.read_to_end(&mut buf).map(|_| buf).ok()?))
                    });

                // Actually send file, same principle as with text message
                if let Some(ref mut writer_mutex) = self.chat_writer {
                    if let Some((file_name, file_contents)) = maybe_file {
                        let owned_writer_mutex = writer_mutex.clone(); // Cheap clone
                        let file_name_clone = file_name.clone(); // Cheap clone
                        let file_contents_clone = file_contents.clone(); // Potentially dangerous clone
                        let sent_message_contents = MessageContents::File {
                            name: file_name_clone,
                            contents: file_contents_clone,
                        };
                        let sent_chat_message = ServerMessage {
                            author: String::from("You"),
                            contents: sent_message_contents,
                        };
                        return Task::perform(
                            async move {
                                let mut writer = owned_writer_mutex.lock().await;
                                writer
                                    .send_file(file_name, file_contents)
                                    .map_ok(|_| sent_chat_message)
                                    .await
                            },
                            move |send_file_result| match send_file_result {
                                Ok(sent_msg) => AppUpdateMessage::ChatPageMessage(
                                    ChatPageMessage::AddMessageToHistory(sent_msg),
                                ),
                                Err(err) => AppUpdateMessage::AddError(err.to_string()),
                            },
                        );
                    }
                }
            }
            ChatPageMessage::Disconnect => {
                if let Some(ref mut writer_mutex) = self.chat_writer {
                    let owned_writer_mutex = writer_mutex.clone(); // Cheap clone
                    return Task::perform(
                        async move {
                            let mut writer = owned_writer_mutex.lock().await;
                            writer.disconnect().await
                        },
                        |disconnect_result| match disconnect_result {
                            Ok(_) => {
                                AppUpdateMessage::ChatPageMessage(ChatPageMessage::ResetChatWriter)
                            }
                            Err(err) => AppUpdateMessage::AddError(err.to_string()),
                        },
                    );
                }
            }
            ChatPageMessage::AddMessageToHistory(msg) => self.chat_messages.push(msg),
            ChatPageMessage::NewChatWriter(new_writer) => {
                let _ = self.chat_writer.replace(new_writer);
            }
            ChatPageMessage::UpdateChatInput(new_chat_input) => self.chat_input = new_chat_input,
            ChatPageMessage::ResetChatWriter => self.chat_writer = None,
        };

        Task::none()
    }

    /// Convenience method to determine if user is in a chat
    pub(crate) fn is_in_chat(&self) -> bool {
        self.chat_writer.is_some()
    }
}
