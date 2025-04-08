mod ws_client_subscription;

use std::fs::File;
use std::io::Read;

use derivative::Derivative;
use futures::channel::mpsc;

use iced::widget::{
    Column, button, column, combo_box, container, row, scrollable, stack, text, text_input,
};
use iced::{Application, Element, Length, Settings, Subscription, Task, Theme, application};
use iced_aw::Card;
use shared_types::messages::{self, ClientMessage};
use ws_client_subscription::{ClientCommand, start_client};

#[derive(Debug, Clone)]
enum AppUpdateMessage {
    AppReady(mpsc::Sender<ClientCommand>),
    UpdateUsername(String),
    UpdatePassword(String),
    UpdateServerAddress(String),
    UpdateAppView(AppView),
    UpdateChatInput(String),
    UpdateTheme(Theme),
    SendMessage(String),
    AttemptSendFile,
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

#[derive(Debug, Derivative)]
#[derivative(Default)]
struct Messenger {
    #[derivative(Default(value = "combo_box::State::new(Theme::ALL.to_vec())"))]
    theme_combobox_state: combo_box::State<Theme>,
    selected_theme: Option<Theme>,
    username: String,
    password: String,
    server_address: String,
    client_channel: Option<mpsc::Sender<ClientCommand>>,
    app_view: AppView,
    messages: Vec<ClientMessage>,
    error_message: Option<String>,
    chat_input: String,
}

fn title(app: &Messenger) -> String {
    String::from("Msger")
}

fn update(app: &mut Messenger, message: AppUpdateMessage) -> iced::Task<AppUpdateMessage> {
    match message {
        AppUpdateMessage::AttemptToConnect => {
            if let Some(ref mut sender) = app.client_channel {
                let try_send = sender.try_send(ClientCommand::Connect(
                    app.username.clone(),
                    app.password.clone(),
                    app.server_address.clone(),
                ));
                match try_send {
                    Ok(_) => {}
                    Err(err) => app.error_message = Some(err.to_string()),
                }
            }
        }

        AppUpdateMessage::SetError(error_message) => {
            app.error_message = Some(error_message);
        }
        AppUpdateMessage::ResetError => {
            app.error_message = None;
        }
        AppUpdateMessage::Disconnect => {
            if let Some(ref mut sender) = app.client_channel {
                let try_send = sender.try_send(ClientCommand::Disconnect);
                match try_send {
                    Ok(_) => {
                        app.messages.clear();
                    }
                    Err(err) => {
                        app.error_message = Some(err.to_string());
                    }
                }
            }
        }
        AppUpdateMessage::UpdateUsername(new_username) => {
            app.username = new_username;
        }
        AppUpdateMessage::UpdateServerAddress(new_server_address) => {
            app.server_address = new_server_address;
        }
        AppUpdateMessage::AppReady(sender) => {
            app.client_channel = Some(sender);
        }
        AppUpdateMessage::UpdateAppView(new_view) => {
            app.app_view = new_view;
        }
        AppUpdateMessage::MessageReceived(new_message) => {
            app.messages.push(new_message);
        }
        AppUpdateMessage::UpdateChatInput(new_chat_input) => {
            app.chat_input = new_chat_input;
        }
        AppUpdateMessage::SendMessage(message) => {
            if let Some(ref mut sender) = app.client_channel {
                let try_send = sender.try_send(ClientCommand::SendMessage(message.clone()));
                match try_send {
                    Ok(_) => {}
                    Err(err) => {
                        app.error_message = Some(err.to_string());
                    }
                }
            }
        }
        AppUpdateMessage::AttemptSendFile => {
            let maybe_file = rfd::FileDialog::new()
                .pick_file()
                // Make into (name, path) tuple
                .and_then(|path_buf| {
                    Some((
                        // If not file indicates logic error with my code and RFD, need something stricter than '?' operator
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

            // Actually send file
            if let Some(ref mut sender) = app.client_channel {
                if let Some(file_name_and_contents) = maybe_file {
                    let try_send = sender.try_send(ClientCommand::SendFile(
                        file_name_and_contents.0, // Name
                        file_name_and_contents.1, // Contents
                    ));
                    match try_send {
                        Ok(_) => {}
                        Err(err) => {
                            app.error_message = Some(err.to_string());
                        }
                    }
                }
            }
        }
        AppUpdateMessage::UpdateTheme(theme) => {
            app.selected_theme = Some(theme);
        }
        AppUpdateMessage::UpdatePassword(password) => {
            app.password = password;
        }
    };
    Task::none()
}

fn view(app: &Messenger) -> iced::Element<AppUpdateMessage> {
    let ui: Element<AppUpdateMessage> = match &app.client_channel {
        Some(_) => match app.app_view {
            AppView::Login => {
                let theme_picker = combo_box(
                    &app.theme_combobox_state,
                    "Theme Selection",
                    app.selected_theme.as_ref(),
                    AppUpdateMessage::UpdateTheme,
                )
                .width(300)
                .padding(5);

                let name_input = text_input("username", &app.username)
                    .on_input(AppUpdateMessage::UpdateUsername)
                    .width(300)
                    .padding(5);
                let password_input = text_input("Password... (Optional)", &app.password)
                    .on_input(AppUpdateMessage::UpdatePassword)
                    .secure(true)
                    .width(300)
                    .padding(5);
                let server_address_input = text_input("wss://...", &app.server_address)
                    .on_input(AppUpdateMessage::UpdateServerAddress)
                    .width(300)
                    .padding(5);

                let submit_message = if !app.username.is_empty() && !app.server_address.is_empty() {
                    Some(AppUpdateMessage::AttemptToConnect)
                } else {
                    None
                };
                let submit_button = button(text("Connect"))
                    .on_press_maybe(submit_message)
                    .padding(5);

                column!(
                    theme_picker,
                    name_input,
                    password_input,
                    server_address_input,
                    submit_button
                )
                .align_x(iced::Alignment::Center)
                .spacing(10)
                .into()
            }
            AppView::Chat => {
                let messages = app.messages.iter().fold(
                    Column::<AppUpdateMessage>::new()
                        .align_x(iced::Alignment::Start)
                        .spacing(2),
                    |col, message| {
                        let message_author = text(format!("{}:", message.author));
                        let message_contents = match &message.contents {
                            messages::MessageContents::Text(txt) => text(txt),
                            // TODO: Add functionality to actually download file
                            messages::MessageContents::File { name, contents } => {
                                text("This is a file wow")
                            }
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

                let chat_input = text_input("Type your message...", &app.chat_input)
                    .on_input(AppUpdateMessage::UpdateChatInput)
                    .on_submit(AppUpdateMessage::SendMessage(app.chat_input.clone()))
                    .padding(5);

                let chat_open_file_button = button(text("Open"))
                    .on_press(AppUpdateMessage::AttemptSendFile)
                    .padding(5);

                let chat_submit_button = button(text("Send"))
                    .on_press(AppUpdateMessage::SendMessage(app.chat_input.clone()))
                    .padding(5);
                let chat_disconnect_button = button(text("Disconnect"))
                    .padding(5)
                    .on_press(AppUpdateMessage::Disconnect);
                let controls = row!(
                    chat_input,
                    chat_submit_button,
                    chat_open_file_button,
                    chat_disconnect_button
                )
                .height(Length::FillPortion(1));

                column!(chat, controls).into()
            }
        },
        None => text("Loading...").into(),
    };

    if let Some(err) = &app.error_message {
        let under_modal = container(ui)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .height(Length::Fill)
            .width(Length::Fill);

        let overlay = Card::new(
            text("An error has occured"),
            column![
                text(err),
                button("Close").on_press(AppUpdateMessage::ResetError)
            ],
        );

        let modal = stack![under_modal, overlay];

        modal.into()
    } else {
        container(ui)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

fn subscription(_app: &Messenger) -> iced::Subscription<AppUpdateMessage> {
    Subscription::run(start_client)
}

fn theme(app: &Messenger) -> Theme {
    app.selected_theme.clone().unwrap_or_default()
}

fn main() -> Result<(), iced::Error> {
    let app = application(title, update, view)
        .subscription(subscription)
        .theme(theme);
    app.run()
}
