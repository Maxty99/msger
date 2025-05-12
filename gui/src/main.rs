#![windows_subsystem = "windows"]
mod components;

use client::ChatSession;
use components::{
    ChatPage, ChatPageMessage, ErrorPopup, ErrorPopupMessage, LoginPage, LoginPageMessage,
};

use futures::StreamExt;
use iced::widget::{container, stack};
use iced::{Element, Length, Subscription, Task, Theme, application};

#[derive(Debug)]
enum AppUpdateMessage {
    LoginPageMessage(LoginPageMessage),
    ErrorPopupMessage(ErrorPopupMessage),
    ChatPageMessage(ChatPageMessage),
    BeginChat(ChatSession, String),
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

fn update(app: &mut Messenger, message: AppUpdateMessage) -> Task<AppUpdateMessage> {
    match message {
        AppUpdateMessage::BeginChat(chat_session, username) => {
            // Start read Stream as a subscription
            // Send the Writer to the chat component to be used there
            let (chat_session_writer, chat_session_reader) = chat_session.split();

            // Bit verbose but works
            let chat_session_read_task =
                Task::stream(chat_session_reader).map(|chat_message_result| {
                    match chat_message_result {
                        Ok(chat_message) => {
                            ChatPageMessage::AddMessageToHistory(chat_message).into()
                        }
                        Err(err) => ErrorPopupMessage::AddError(err.to_string()).into(),
                    }
                });
            let chat_worker_update_stream =
                Task::stream(ChatPage::init_worker(username, chat_session_writer));
            Task::batch(vec![chat_session_read_task, chat_worker_update_stream])
        }

        // Component Updaters
        AppUpdateMessage::LoginPageMessage(login_page_message) => {
            app.login_page.update(login_page_message).into()
        }
        AppUpdateMessage::ErrorPopupMessage(error_popup_message) => {
            app.error_popup.update(error_popup_message).into()
        }
        AppUpdateMessage::ChatPageMessage(chat_page_message) => {
            app.chat_page.update(chat_page_message).into()
        }
    }
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
