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
