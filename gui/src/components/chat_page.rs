mod chat_worker;
use anyhow::{Context, Result, bail};
use chat_worker::ChatSender;
use client::ChatWrite;
use futures::Stream;
use iced::{
    Length, Task,
    widget::{button, column, row, scrollable, text, text_input},
};
use shared_types::messages::{MessageContents, ServerMessage};
use size::Size as PrettyFileSize;
use std::{fs::File, io::Read};

use crate::AppUpdateMessage;

use super::ErrorPopupMessage;

//TODO: Maybe make this adjustable?
const MAXIMUM_FILE_SIZE_BYTES: PrettyFileSize = PrettyFileSize::from_const(1_073_741_824); // 1 GB

#[derive(Debug, Default)]
pub(crate) struct ChatPage {
    chat_messages: Vec<ServerMessage>,
    chat_input: String,
    chat_sender: Option<ChatSender>,
}

#[derive(Debug, Clone)]
pub(crate) enum ChatPageMessage {
    ResetChatWorker,
    WorkerReady(ChatSender),
    UpdateChatInput(String),
    SendMessage(String),
    AddMessageToHistory(ServerMessage),
    AttemptSendFile,
    Disconnect,
}

impl From<ChatPageMessage> for AppUpdateMessage {
    fn from(val: ChatPageMessage) -> Self {
        AppUpdateMessage::ChatPageMessage(val)
    }
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
                    MessageContents::File { name: _, contents: _ } => text("This is a file wow"),
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

    pub(crate) fn update(&mut self, message: ChatPageMessage) -> impl Into<Task<AppUpdateMessage>> {
        match message {
            ChatPageMessage::SendMessage(message_text) => {
                if let Some(ref mut sender) = self.chat_sender {
                    match sender.try_send(client::ClientMessage::text(message_text)) {
                        Ok(_) => {
                            self.chat_input.clear();
                            return Task::none();
                        },
                        Err(err) => {
                            return Task::done(ErrorPopupMessage::AddError(err.to_string()).into());
                        }
                    }
                }
            }
            ChatPageMessage::AttemptSendFile => {
                let maybe_file = rfd::FileDialog::new().pick_file();

                let process_file_result: Result<(String, Vec<u8>)> = maybe_file
                    .context("File dialog did not return a ")
                    // Make into (name, path) tuple
                    .and_then(|path_buf| {
                        Ok((
                            // If not file indicates logic error with my code and RFD
                            // need something stricter than '?' operator so using expect
                            path_buf
                                .file_name()
                                .expect("file dialog to only give file")
                                .to_string_lossy() // Not big deal if name is a bit mangled
                                .into(),
                            File::open(path_buf)?,
                        ))
                    })
                    // Make into (name, file contents) tuple
                    .and_then(|(filename, mut file)| {
                        
                        let file_metadata = file
                            .metadata()
                            .context("Could not get file metadata to check for file size")?;
                        // Need some sort of protection against overflowing memory
                        // by accident by selecting a very large file
                        let file_size = PrettyFileSize::from_bytes(file_metadata.len());
                        if file_size <= MAXIMUM_FILE_SIZE_BYTES {
                            let mut buf = vec![];
                            let _ = file.read_to_end(&mut buf).context("Error while reading file")?;
                            Ok((filename, buf))
                        } else {
                            bail!("Selected file was too large: {file_size} (Maximum: {MAXIMUM_FILE_SIZE_BYTES})");
                        }
                    });

                if let Some(ref mut sender) = self.chat_sender {
                    
                    match process_file_result.and_then(|(filename, file_contents)| {
                        sender.try_send(client::ClientMessage::file(filename, file_contents)).map_err(anyhow::Error::from)
                    }) {
                        Ok(_) => return Task::none(),
                        Err(err) => return Task::done(ErrorPopupMessage::AddError(err.to_string()).into()),
                    }
                }
            }
            ChatPageMessage::Disconnect => {
                if let Some(ref mut sender) = self.chat_sender {
                    match sender.try_send(client::ClientMessage::disconnect_message()) {
                        Ok(_) => return Task::none(),
                        Err(err) => {
                            return Task::done(ErrorPopupMessage::AddError(err.to_string()).into());
                        }
                    }
                }
            }

            // Simple Updaters
            ChatPageMessage::AddMessageToHistory(msg) => self.chat_messages.push(msg),
            ChatPageMessage::UpdateChatInput(new_chat_input) => self.chat_input = new_chat_input,
            ChatPageMessage::ResetChatWorker => {
                self.chat_sender = None;
                self.chat_messages.clear();
                self.chat_input.clear();
            }
            ChatPageMessage::WorkerReady(chat_worker_sender) => {
                self.chat_sender = Some(chat_worker_sender)
            }
        };

        Task::none()
    }

    /// Convenience method to determine if user is in a chat
    pub(crate) fn is_in_chat(&self) -> bool {
        self.chat_sender.is_some()
    }

    pub(crate) fn init_worker(
        username: String,
        chat_session_writer: impl ChatWrite,
    ) -> impl Stream<Item = AppUpdateMessage> {
        chat_worker::start_chat_worker(chat_session_writer, username)
    }
}
