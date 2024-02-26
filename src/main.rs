use std::fs::File;
use std::io::Read;

use clap::Parser;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::get_access_token;
use crate::backup::{compress_volume, generate_file_name};
use crate::cli::Args;
use crate::upload::upload_file;

mod backup;
mod cli;
mod auth;
mod upload;

#[derive(Debug, Deserialize)]
struct Config {
    mc_server_name: String,
    gcs_credentials_path: String,
    gcs_bucket_name: String,
    volumes_path: String,
    smp_uuid: Uuid,
    cmp_uuid: Uuid,
}

impl Config {
    fn load() -> anyhow::Result<Self> {
        let config_string = std::fs::read_to_string("config.json")?;
        serde_json::from_str(&config_string).map_err(Into::into)
    }
}

#[tokio::main]
async fn main() {
    log::info!("Starting pterobackup...");
    let cli_args = Args::parse();

    init_logger(cli_args.verbose);
    log::debug!("Parsed CLI args: {:?}", cli_args);

    let config = Config::load().expect("Failed to load config");
    log::debug!("Loaded config: {:?}", config);

    let backup_file_name = generate_file_name(config.mc_server_name.as_str(), &cli_args.server);
    let access_token = get_access_token(&config.gcs_credentials_path).await.expect("Failed to get access token");
    log::debug!("Access token: {}", access_token);
    
    compress_volume(
        &cli_args.server,
        &cli_args.exclude,
        &config,
        backup_file_name.as_str(),
    )
    .expect("Failed to create tar.gz file");
    log::debug!("Backup file created: {}", &backup_file_name);

    let mut compressed_backup = File::open(&backup_file_name).expect("Failed to open backup file");
    let buffer = create_buffer(&mut compressed_backup).expect("Failed to create buffer");
    log::debug!("Buffer with lenth {} created.", buffer.len());

    upload_file(config.mc_server_name.as_str(), &cli_args.server, config.gcs_bucket_name.as_str(), backup_file_name.as_str(), &access_token).await.expect("Failed to upload file");
    log::info!("Backup uploaded to GCS bucket: {}", config.gcs_bucket_name);

    delete_local_file(&backup_file_name).expect("Failed to delete local file");
    log::debug!("Local backup file deleted.");
}

fn create_buffer(file: &mut File) -> anyhow::Result<Vec<u8>> {
    let metadata = file.metadata()?;
    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut buffer)
        .expect("Failed to read file into buffer");

    Ok(buffer)
}

fn delete_local_file(file_name: &str) -> anyhow::Result<()> {
    std::fs::remove_file(file_name)?;
    Ok(())
}

fn init_logger(verbose: u8) {
    env_logger::Builder::new()
        .filter_level(match verbose {
            0 => log::LevelFilter::Error, // Default to only showing errors
            1 => log::LevelFilter::Warn,  // -v or --verbose shows warnings and errors
            2 => log::LevelFilter::Info,  // -vv or --verbose --verbose shows info and above
            _ => log::LevelFilter::Debug, // -vvv or more shows debug and above
        })
        .init();
}
