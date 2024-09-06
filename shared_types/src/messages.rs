use crate::base64_serialize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MessageContents {
    Text(String),
    #[serde(with = "base64_serialize")]
    File(Vec<u8>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientMessage {
    pub author: String,
    pub contents: MessageContents,
}
