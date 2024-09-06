use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    time::Duration,
};

use clap::{Args, Parser};
use serde::{Deserialize, Serialize};

// Helper defualt functions for serde and clap

const fn default_allow_files() -> bool {
    true
}

const fn default_message_timout() -> Duration {
    Duration::from_secs(5)
}

const fn default_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
}

const fn default_port() -> u16 {
    2004
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

    #[arg(short = 'i', long = "ipaddr", default_value = "127.0.0.1")]
    // Set the IP that the server will bind to
    #[serde(default = "default_ip")]
    pub(crate) ip_addr: IpAddr,

    #[arg(short = 'p', long = "port", default_value = "2004")]
    // Set the port that the server will bind to
    #[serde(default = "default_port")]
    pub(crate) port: u16,
}

#[derive(Parser, Debug)]
#[command(about = "Official/Refrence implementation for <Project Name>", long_about = None)]
pub(crate) struct ClapArgConfig {
    #[command(flatten)]
    pub(crate) server_config: ServerConfig,

    #[arg(short = 'c', long = "config", group = "config_file")]
    /// Optional path to a toml file used to configure the server, overrides any other command line args
    pub(crate) server_config_file: Option<PathBuf>,

    #[arg(
        short = 'e',
        long = "editor",
        requires = "config_file",
        default_value = "false"
    )]
    /// Option to enter special config editor mode, must have provided a server config file path
    pub(crate) server_config_file_editor_flag: bool,
}
