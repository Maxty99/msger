use std::{net::IpAddr, path::PathBuf, time::Duration};

use clap::{Args, Parser};
use serde::{Deserialize, Serialize};

// Helper defualt functions for serde and clap

const fn default_allow_files() -> bool {
    true
}

const fn default_message_timout() -> Duration {
    Duration::from_secs(5)
}

#[derive(Deserialize, Serialize, Args, Debug)]
/// # Server Configuration
/// This struct defines the possible configuration options of the server
///
/// You may set these options from the command-line or from a TOML file
/// You may also use the config editor feature of the server to edit this file before starting the server
pub(crate) struct ServerConfig {
    #[arg(short = 'b', long = "banned")]
    /// Optional list of users that will not be able to initiate a connection with the server
    #[serde(default)]
    pub(crate) banned_users: Vec<IpAddr>,

    #[arg(short = 'a', long = "auth")]
    /// Optional authentication which will be used to encrypt outgoing data
    #[serde(default)]
    pub(crate) auth: Option<String>,

    #[arg(short = 'f', long = "allow-files", default_value = "true")]
    /// Allow sending files to the chat
    #[serde(default = "default_allow_files")]
    pub(crate) allow_files: bool,

    #[clap(value_parser = humantime::parse_duration, default_value = "5s")]
    #[arg(short = 't', long = "message-timeout")]
    /// The time span within which a user may not send more than 5 messages, and if they do they will be timed out
    #[serde(default = "default_message_timout")]
    pub(crate) message_timeout: Duration,
}

#[derive(Parser, Debug)]
#[command(about = "Official/Refrence implementation for <Project Name>", long_about = None)]
pub(crate) struct ClapArgConfig {
    #[command(flatten)]
    pub(crate) server_config: ServerConfig,

    #[arg(short = 'c', long = "config")]
    /// Optional path to a toml file used to configure the server, overrides any other command line args
    pub(crate) server_config_file: Option<PathBuf>,
}
