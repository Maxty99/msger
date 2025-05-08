use iced::{
    Length, Task,
    widget::{button, column, container, row, stack, text},
};
use iced_aw::card;

use crate::AppUpdateMessage;

#[derive(Debug, Default)]
pub(crate) struct ErrorPopup {
    error_messages: Vec<String>,
    is_list_visible: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum ErrorPopupMessage {
    AddError(String),
    RemoveError(usize),
    ToggleExpand,
}

impl ErrorPopup {
    //TODO: Make this prettier
    pub(crate) fn view(&self) -> iced::Element<ErrorPopupMessage> {
        let toggle_errorlist_button = container(
            button(text("Toggle Errors"))
                .on_press(ErrorPopupMessage::ToggleExpand)
                .padding(5)
                .width(Length::Shrink)
                .height(Length::Shrink),
        )
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_top(Length::Fill)
        .align_right(Length::Fill);

        let error_list: iced::Element<ErrorPopupMessage> = if self.error_messages.is_empty() {
            text("No recent errors...").center().into()
        } else {
            self.error_messages
                .iter()
                .enumerate()
                .fold(column!(), |col, (idx, err_message)| {
                    col.push(row![
                        text(err_message),
                        button(text("Clear")).on_press(ErrorPopupMessage::RemoveError(idx))
                    ])
                })
                .into()
        };

        let error_card = container(card(text("Errors"), error_list))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill);

        if self.is_list_visible {
            stack![error_card, toggle_errorlist_button].into()
        } else {
            stack![toggle_errorlist_button].into()
        }
    }

    pub(crate) fn update(&mut self, message: ErrorPopupMessage) -> iced::Task<AppUpdateMessage> {
        match message {
            ErrorPopupMessage::AddError(new_err) => self.error_messages.push(new_err),
            ErrorPopupMessage::RemoveError(idx) => _ = self.error_messages.remove(idx),
            ErrorPopupMessage::ToggleExpand => self.is_list_visible = !self.is_list_visible,
        }
        Task::none()
    }
}
