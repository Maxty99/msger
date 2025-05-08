#![windows_subsystem = "windows"]
mod components;

use std::sync::Arc;

use client::client::ChatSession;
use components::{
    ChatPage, ChatPageMessage, ErrorPopup, ErrorPopupMessage, LoginPage, LoginPageMessage,
};

use futures::StreamExt;
use iced::widget::{container, stack};
use iced::{Element, Length, Subscription, Task, Theme, application};
use tokio::sync::Mutex;

#[derive(Debug)]
enum AppUpdateMessage {
    LoginPageMessage(LoginPageMessage),
    ErrorPopupMessage(ErrorPopupMessage),
    ChatPageMessage(ChatPageMessage),
    BeginChat(ChatSession),
    AddError(String),
}

#[derive(Debug, Default)]
struct Messenger {
    login_page: LoginPage,
    error_popup: ErrorPopup,
    chat_page: ChatPage,
}

fn title(_app: &Messenger) -> String {
    String::from("Msger")
}

fn update(app: &mut Messenger, message: AppUpdateMessage) -> iced::Task<AppUpdateMessage> {
    match message {
        AppUpdateMessage::AddError(error_message) => {
            // A little jank but just redirects to the popup component.
            // Allows all my components to use the AppUpdateMessage enum
            // to push errors to the list without worrying about how it gets there
            let _ = app // Ignore task as always Task::none()
                .error_popup
                .update(ErrorPopupMessage::AddError(error_message));
        }
        AppUpdateMessage::BeginChat(chat_session) => {
            // Start read Stream as a subscription
            // Send the Writer to the chat component to be used there
            let (chat_session_write, chat_session_read) = chat_session.split();

            // Bit verbose but works
            let chat_session_read_task = Task::stream(chat_session_read.map(
                |chat_message_result| match chat_message_result {
                    Ok(chat_message) => AppUpdateMessage::ChatPageMessage(
                        ChatPageMessage::AddMessageToHistory(chat_message),
                    ),
                    Err(err) => AppUpdateMessage::ErrorPopupMessage(ErrorPopupMessage::AddError(
                        err.to_string(),
                    )),
                },
            ));
            // This particular update call will always return Task::none()
            // So we just ignore it :)
            let _ = app
                .chat_page
                .update(ChatPageMessage::NewChatWriter(Arc::new(Mutex::new(
                    chat_session_write,
                ))));
            return chat_session_read_task;
        }

        // Component Updaters
        AppUpdateMessage::LoginPageMessage(login_page_message) => {
            return app.login_page.update(login_page_message);
        }
        AppUpdateMessage::ErrorPopupMessage(error_popup_message) => {
            return app.error_popup.update(error_popup_message);
        }
        AppUpdateMessage::ChatPageMessage(chat_page_message) => {
            return app.chat_page.update(chat_page_message);
        }
    };
    Task::none()
}

fn view(app: &Messenger) -> iced::Element<AppUpdateMessage> {
    let main_ui: Element<AppUpdateMessage> = if app.chat_page.is_in_chat() {
        app.chat_page.view().map(AppUpdateMessage::ChatPageMessage)
    } else {
        app.login_page
            .view()
            .map(AppUpdateMessage::LoginPageMessage)
    };
    let error_popup = app
        .error_popup
        .view()
        .map(AppUpdateMessage::ErrorPopupMessage);

    container(stack![main_ui, error_popup])
        .center(Length::Fill)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn subscription(_app: &Messenger) -> iced::Subscription<AppUpdateMessage> {
    // Subscription::run(start_client)
    Subscription::none()
}

fn theme(app: &Messenger) -> Theme {
    // Temporary, see comment for this function in the LoginPage source
    app.login_page.get_selected_theme()
}

fn main() -> Result<(), iced::Error> {
    let app = application(title, update, view)
        .subscription(subscription)
        .theme(theme);
    app.run()
}
