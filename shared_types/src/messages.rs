use crate::base64_serialize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageContents {
    Text(String),

    File {
        name: String,
        #[serde(with = "base64_serialize")]
        contents: Vec<u8>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerMessage {
    pub author: String,
    pub contents: MessageContents,
}

impl ServerMessage {
    #[inline]
    pub fn text(author: impl ToString, contents: impl ToString) -> Self {
        Self {
            author: author.to_string(),
            contents: MessageContents::Text(contents.to_string()),
        }
    }

    #[inline]
    pub fn file(
        author: impl ToString,
        filename: impl ToString,
        file_contents: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            author: author.to_string(),
            contents: MessageContents::File {
                name: filename.to_string(),
                contents: file_contents.into(),
            },
        }
    }

    pub fn disconnect_message() -> Self {
        ServerMessage {
            author: String::from("Server"),
            contents: MessageContents::Text(String::from("You have been disconnected...")),
        }
    }
}
