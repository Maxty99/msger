use client::connect;
use derivative::Derivative;
use futures::TryFutureExt;
use iced::{
    Length, Task, Theme,
    widget::{button, column, combo_box, container, text, text_input},
};

use crate::AppUpdateMessage;

use super::ErrorPopupMessage;

#[derive(Debug, Derivative)]
#[derivative(Default)]
pub(crate) struct LoginPage {
    #[derivative(Default(value = "combo_box::State::new(Theme::ALL.to_vec())"))]
    theme_combobox_state: combo_box::State<Theme>,
    selected_theme: Theme,
    username: String,
    password: String,
    server_addr: String,
}

#[derive(Debug, Clone)]
pub(crate) enum LoginPageMessage {
    AttemptToConnect,
    UpdateUsername(String),
    UpdatePassword(String),
    UpdateServerAddress(String),
    UpdateTheme(Theme),
}

impl From<LoginPageMessage> for AppUpdateMessage {
    fn from(val: LoginPageMessage) -> Self {
        AppUpdateMessage::LoginPageMessage(val)
    }
}

impl LoginPage {
    pub(crate) fn view(&self) -> iced::Element<LoginPageMessage> {
        let theme_picker = combo_box(
            &self.theme_combobox_state,
            "Theme Selection",
            Some(&self.selected_theme),
            LoginPageMessage::UpdateTheme,
        )
        .width(300)
        .padding(5);

        let name_input = text_input("username", &self.username)
            .on_input(LoginPageMessage::UpdateUsername)
            .width(300)
            .padding(5);

        let password_input = text_input("Password... (Optional)", &self.password)
            .on_input(LoginPageMessage::UpdatePassword)
            .secure(true)
            .width(300)
            .padding(5);

        let server_address_input = text_input("wss://...", &self.server_addr)
            .on_input(LoginPageMessage::UpdateServerAddress)
            .width(300)
            .padding(5);

        let submit_message = if !self.username.is_empty() && !self.server_addr.is_empty() {
            Some(LoginPageMessage::AttemptToConnect)
        } else {
            None
        };
        let submit_button = button(text("Connect"))
            .on_press_maybe(submit_message)
            .padding(5);

        container(
            column!(
                theme_picker,
                name_input,
                password_input,
                server_address_input,
                submit_button
            )
            .align_x(iced::Alignment::Center)
            .spacing(10),
        )
        .center(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
    pub(crate) fn update(
        &mut self,
        message: LoginPageMessage,
    ) -> impl Into<Task<AppUpdateMessage>> {
        match message {
            // Tell main app to initiate connection
            LoginPageMessage::AttemptToConnect => {
                // Needed to be able to pass as a task up to the main app
                // Not happening often
                let function_owned_user = self.username.clone();
                let function_owned_addr = self.server_addr.clone();
                let function_owned_pass = if self.password.is_empty() {
                    None
                } else {
                    Some(self.password.clone())
                };

                let connection_future = connect(
                    // Double clone since I use it twice, here and
                    // in the Ok variant that I send back to the main app
                    // I can't keep track of the username otherwise
                    function_owned_user.clone(),
                    function_owned_pass,
                    function_owned_addr,
                )
                .map_ok(|chat_session| (chat_session, function_owned_user));

                return Task::perform(
                    connection_future,
                    |connection_result| match connection_result {
                        Ok((chat_session, username)) => {
                            AppUpdateMessage::BeginChat(chat_session, username)
                        }
                        Err(err) => ErrorPopupMessage::AddError(err.to_string()).into(),
                    },
                );
            }

            // Basic updaters
            LoginPageMessage::UpdateUsername(new_username) => self.username = new_username,
            LoginPageMessage::UpdatePassword(new_password) => self.password = new_password,
            LoginPageMessage::UpdateServerAddress(new_addr) => self.server_addr = new_addr,
            LoginPageMessage::UpdateTheme(new_theme) => self.selected_theme = new_theme,
        }
        Task::none()
    }

    // TODO: Might get rid of this in the future for a more robust Theme
    // controller with persistence features using https://github.com/rust-cli/confy
    pub(crate) fn get_selected_theme(&self) -> Theme {
        self.selected_theme.clone()
    }
}
