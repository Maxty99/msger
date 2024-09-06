mod ws_client_subscription;

use futures::channel::mpsc;

use iced::executor::Default as DefaultExecutor;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Application, Command, Element, Length, Settings, Theme};
use iced_aw::widgets::Modal;
use iced_aw::Card;
use shared_types::messages::{self, ClientMessage};
use ws_client_subscription::{start_client, ClientCommand};

#[derive(Debug, Clone)]
enum AppUpdateMessage {
    AppReady(mpsc::Sender<ClientCommand>),
    UpdateUsername(String),
    UpdateServerAddress(String),
    UpdateAppView(AppView),
    UpdateChatInput(String),
    SendMessage(String),
    AttemptToConnect,
    MessageReceived(ClientMessage),
    SetError(String),
    ResetError,
    Disconnect,
}
#[derive(Debug, Default, Clone, Copy)]
enum AppView {
    #[default]
    Login,
    Chat,
}

#[derive(Debug, Default)]
struct Messenger {
    username: String,
    server_address: String,
    client_channel: Option<mpsc::Sender<ClientCommand>>,
    app_view: AppView,
    messages: Vec<ClientMessage>,
    error_message: Option<String>,
    chat_input: String,
}

impl Application for Messenger {
    type Executor = DefaultExecutor;

    type Message = AppUpdateMessage;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (Messenger::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("Msger")
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            AppUpdateMessage::AttemptToConnect => {
                if let Some(ref mut sender) = self.client_channel {
                    let try_send = sender.try_send(ClientCommand::Connect(
                        self.username.clone(),
                        self.server_address.clone(),
                    ));
                    match try_send {
                        Ok(_) => {}
                        Err(err) => {
                            self.error_message = Some(err.to_string());
                        }
                    }
                }
            }

            AppUpdateMessage::SetError(error_message) => {
                self.error_message = Some(error_message);
            }
            AppUpdateMessage::ResetError => {
                self.error_message = None;
            }
            AppUpdateMessage::Disconnect => {
                if let Some(ref mut sender) = self.client_channel {
                    let try_send = sender.try_send(ClientCommand::Disconnect);
                    match try_send {
                        Ok(_) => {
                            self.messages.clear();
                        }
                        Err(err) => {
                            self.error_message = Some(err.to_string());
                        }
                    }
                }
            }
            AppUpdateMessage::UpdateUsername(new_username) => {
                self.username = new_username;
            }
            AppUpdateMessage::UpdateServerAddress(new_server_address) => {
                self.server_address = new_server_address;
            }
            AppUpdateMessage::AppReady(sender) => {
                self.client_channel = Some(sender);
            }
            AppUpdateMessage::UpdateAppView(new_view) => {
                self.app_view = new_view;
            }
            AppUpdateMessage::MessageReceived(new_message) => {
                self.messages.push(new_message);
            }
            AppUpdateMessage::UpdateChatInput(new_chat_input) => {
                self.chat_input = new_chat_input;
            }
            AppUpdateMessage::SendMessage(message) => {
                if let Some(ref mut sender) = self.client_channel {
                    let try_send = sender.try_send(ClientCommand::SendMessage(message.clone()));
                    match try_send {
                        Ok(_) => {}
                        Err(err) => {
                            self.error_message = Some(err.to_string());
                        }
                    }
                }
            }
        };
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let ui: Element<AppUpdateMessage> = match &self.client_channel {
            Some(_) => match self.app_view {
                AppView::Login => {
                    let name_input = text_input("username", &self.username)
                        .on_input(AppUpdateMessage::UpdateUsername)
                        .width(300)
                        .padding(5);
                    let server_address_input = text_input("wss://...", &self.server_address)
                        .on_input(AppUpdateMessage::UpdateServerAddress)
                        .width(300)
                        .padding(5);

                    let submit_message =
                        if !self.username.is_empty() && !self.server_address.is_empty() {
                            Some(AppUpdateMessage::AttemptToConnect)
                        } else {
                            None
                        };
                    let submit_button = button(text("Connect"))
                        .on_press_maybe(submit_message)
                        .padding(5);

                    column!(name_input, server_address_input, submit_button)
                        .align_items(iced::Alignment::Center)
                        .spacing(10)
                        .into()
                }
                AppView::Chat => {
                    let messages = self.messages.iter().fold(
                        Column::<AppUpdateMessage>::new()
                            .align_items(iced::Alignment::Start)
                            .spacing(2),
                        |col, message| {
                            let message_author = text(format!("{}:", message.author));
                            let message_contents = match &message.contents {
                                messages::MessageContents::Text(txt) => text(txt),
                                messages::MessageContents::File(_) => text("This is a file wow"),
                            };
                            let message_row = row!(message_author, message_contents)
                                .spacing(2)
                                .align_items(iced::Alignment::End);
                            col.push(message_row)
                        },
                    );
                    let chat = scrollable(messages)
                        .height(Length::FillPortion(9))
                        .width(Length::Fill);

                    let chat_input = text_input("Type your message...", &self.chat_input)
                        .on_input(AppUpdateMessage::UpdateChatInput)
                        .on_submit(AppUpdateMessage::SendMessage(self.chat_input.clone()))
                        .padding(5);

                    let chat_submit_button = button(text("Send"))
                        .on_press(AppUpdateMessage::SendMessage(self.chat_input.clone()))
                        .padding(5);
                    let chat_disconnect_button = button(text("Disconnect"))
                        .padding(5)
                        .on_press(AppUpdateMessage::Disconnect);
                    let controls = row!(chat_input, chat_submit_button, chat_disconnect_button)
                        .height(Length::FillPortion(1));

                    column!(chat, controls).into()
                }
            },
            None => text("Loading...").into(),
        };

        if let Some(err) = &self.error_message {
            let under_modal = container(ui)
                .center_x()
                .center_y()
                .height(Length::Fill)
                .width(Length::Fill);
            let overlay = Card::new(text("An error has occured"), text(err));

            let modal = Modal::new(under_modal, Some(overlay))
                .backdrop(AppUpdateMessage::ResetError)
                .on_esc(AppUpdateMessage::ResetError);
            modal.into()
        } else {
            container(ui)
                .center_x()
                .center_y()
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        start_client()
    }
}

fn main() -> Result<(), iced::Error> {
    Messenger::run(Settings::default())
}
