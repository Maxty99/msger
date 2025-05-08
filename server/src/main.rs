mod config;
mod error;
mod server;

use clap::Parser;
use config::ClapArgConfig;
use error::ServerError;
use log::*;
use server::Server;

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    env_logger::init();

    // TODO(MT): Setup signal handling

    info!("Begin initializing server from config");

    let args = ClapArgConfig::parse();

    debug!("Initialized with command line args: {args:?}");

    let server_config = if let Some(path_to_config) = args.server_config_file {
        debug!("User specified config file, attempting to read config file");

        let config_string_result = std::fs::read_to_string(path_to_config).map_err(|error| {
            error!("Error has occured while trying to read config file: {error:?}");
            ServerError::ReadConfigError(error)
        })?;

        debug!("Successfully read config file, attempting to parse");
        warn!("Config file will override any command line args");

        toml::from_str(&config_string_result).map_err(|error| {
            error!("Error has occured while parsing config file: {error:?}");
            error
        })?
    } else {
        info!("No config file specified");
        args.server_config
    };

    info!("Successfully loaded server config");

    if args.server_config_file_editor_flag {
        info!("Starting the config editor");
        //TODO: Create config editor functionality
        todo!()
    } else {
        let server = Server::new(server_config);
        server.run_server().await
    }
}
