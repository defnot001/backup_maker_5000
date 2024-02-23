#![allow(unused, dead_code)]

use std::borrow::Cow;
use std::fmt::Display;
use std::fs::{File, Metadata, ReadDir};
use std::io::Read;
use std::ops::ControlFlow;
use clap::{Parser, ValueEnum};
use flate2::write::GzEncoder;
use serde::Deserialize;
use google_cloud_storage::client::{ClientConfig, Client as GCSClient};
use google_cloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::http::buckets::list::ListBucketsRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(
    name = "pterobackup",
    version = "1.0",
    about = "A simple tool to take a backup on a pterodactyl server and then upload it to a gcs bucket.",
    long_about = None,
)]
struct Args {
    /// Which server to backup
    #[arg(short = 's', long = "server", value_enum)]
    server: ServerType,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ServerType {
    /// KiwiTech survival server
    Smp,
    /// KiwiTech creative server
    Cmp,
}

impl Display for ServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerType::Smp => write!(f, "smp"),
            ServerType::Cmp => write!(f, "cmp"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
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
    let args = Args::parse();
    let config = Config::load().expect("Failed to load config");

    let gcs_client = generate_gcs_client(config.gcs_credentials_path.as_str()).await.expect("Failed to generate GCS Client");

    let volume_path = match args.server {
        ServerType::Smp => format!("{}/{}", config.volumes_path, config.smp_uuid),
        ServerType::Cmp => format!("{}/{}", config.volumes_path, config.cmp_uuid),
    };

    let volume_dir = std::fs::read_dir(volume_path).expect("Failed to read volume directory");
    let (mut tar_gz, filename) = gzip_volume(volume_dir, &args.server).expect("Failed to create tar.gz file");
    let metadata = tar_gz.metadata().expect("Failed to get metadata");

    let mut buffer: Vec<u8> = vec![0; metadata.len() as usize];
    tar_gz.read_exact(&mut buffer).expect("Failed to read file. Buffer too small?");

    let uploaded = gcs_client.upload_object(&UploadObjectRequest {
        bucket: config.gcs_bucket_name.clone(),
        ..Default::default()
    }, buffer, &UploadType::Simple(get_media(metadata, filename, &args.server))).await.expect("Failed to upload file");
}

async fn generate_gcs_client(credentials_file_path: &str) -> anyhow::Result<GCSClient> {
    let credentials_file = CredentialsFile::new_from_file(credentials_file_path.to_string()).await?;
    let gcs_config = ClientConfig::default().with_credentials(credentials_file).await?;
    Ok(GCSClient::new(gcs_config))
}
fn gzip_volume(volume_dir: ReadDir, server_type: &ServerType) -> anyhow::Result<(File, String)> {
    let now = chrono::Utc::now();
    let file_name = format!("{}_KiwiTech_{}.tar", now.format("%Y-%m-%d"), server_type.to_string().to_uppercase());

    let tar_gz = File::create(file_name.as_str())?;
    let mut encoder = GzEncoder::new(tar_gz, flate2::Compression::default());

    Ok((encoder.finish()?, file_name))
}

fn get_media(metadata: Metadata, filename: String, server_type: &ServerType) -> Media {
    let mut media = Media::new(format!("KiwiTech/{}/{}", server_type.to_string().to_uppercase(), filename.as_str()));
    media.content_type = Cow::from("application/gzip");
    media.content_length = Some(metadata.len());

    media
}