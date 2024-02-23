#![allow(unused, dead_code)]

use std::borrow::Cow;
use std::fmt::Display;
use std::fs::{File, Metadata, OpenOptions, ReadDir};
use std::io::Read;
use std::ops::ControlFlow;
use std::path::Path;
use clap::{Parser, ValueEnum};
use flate2::write::GzEncoder;
use serde::Deserialize;
use google_cloud_storage::client::{ClientConfig, Client as GCSClient};
use google_cloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::http::buckets::list::ListBucketsRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use serde_json::Value;
use uuid::Uuid;
use walkdir::WalkDir;

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

    /// Verbose mode (-v, --verbose)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Name of the folder to exclude from the backup
    #[arg(short = 'e', long = "exclude")]
    exclude: Option<String>,
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

    env_logger::Builder::new()
        .filter_level(match args.verbose {
            0 => log::LevelFilter::Error, // Default to only showing errors
            1 => log::LevelFilter::Warn,  // -v or --verbose shows warnings and errors
            2 => log::LevelFilter::Info,  // -vv or --verbose --verbose shows info and above
            _ => log::LevelFilter::Debug, // -vvv or more shows debug and above
        })
        .init();

    log::info!("Starting pterobackup...");

    let config = Config::load().expect("Failed to load config");
    let gcs_client = generate_gcs_client(config.gcs_credentials_path.as_str()).await.expect("Failed to generate GCS Client");

    let volume_path = match args.server {
        ServerType::Smp => format!("{}/{}", config.volumes_path, config.smp_uuid),
        ServerType::Cmp => format!("{}/{}", config.volumes_path, config.cmp_uuid),
    };

    let file_name = compress_volume(volume_path, &args.server, &args.exclude).expect("Failed to create tar.gz file");

    let mut compressed_backup = File::open(file_name).expect("Failed to open backup file");
    let metadata = compressed_backup.metadata().expect("Failed to get file metadata");

    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    compressed_backup.read_to_end(&mut buffer).expect("Failed to read file into buffer");

    // let uploaded = gcs_client.upload_object(&UploadObjectRequest {
    //     bucket: config.gcs_bucket_name.clone(),
    //     ..Default::default()
    // }, buffer, &UploadType::Simple(get_media(metadata, file_name, &args.server))).await.expect("Failed to upload file");
}

async fn generate_gcs_client(credentials_file_path: &str) -> anyhow::Result<GCSClient> {
    let credentials_file = CredentialsFile::new_from_file(credentials_file_path.to_string()).await?;
    let gcs_config = ClientConfig::default().with_credentials(credentials_file).await?;
    Ok(GCSClient::new(gcs_config))
}
fn compress_volume(volume_dir: String, server_type: &ServerType, exclude: &Option<String>) -> anyhow::Result<String> {
    let now = chrono::Utc::now();
    let file_name = format!("{}_KiwiTech_{}.tar.gz", now.format("%Y-%m-%d"), server_type.to_string().to_uppercase());

    create_tar_gz(volume_dir, file_name.clone(), exclude)?;
    Ok(file_name)
}

fn create_tar_gz<P: AsRef<Path>>(source_dir: P, output_file: P, exclude: &Option<String>) -> anyhow::Result<()> {
    log::info!("Creating tar.gz file: {:?}", output_file.as_ref());
    let tar_file = File::create(output_file.as_ref())?;
    let enc = GzEncoder::new(tar_file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(enc);

    let source_dir = source_dir.as_ref().canonicalize()?;
    log::debug!("Canonicalized source directory: {:?}", source_dir);

    for entry in WalkDir::new(&source_dir) {
        let entry = entry?;
        let path = entry.path();

        if let Some(ref exclude_folder) = exclude {
            let relative_path = path.strip_prefix(&source_dir).unwrap_or(path);
            if path.to_str().map_or(false, |s| s.contains(exclude_folder)) {
                log::info!("Excluding: {:?}", relative_path);
                continue;
            }
        }

        if let Ok(relative_path) = path.strip_prefix(&source_dir) {
            if relative_path.components().next().is_some() {
                if path.is_file() {
                    log::debug!("Adding file to tar.gz: {:?}", relative_path);
                    tar_builder.append_path_with_name(path, relative_path)?;
                } else if !relative_path.as_os_str().is_empty() {
                    log::debug!("Adding directory to tar.gz: {:?}", relative_path);
                    tar_builder.append_dir(relative_path, path)?;
                } else {
                    log::debug!("Skipping empty path relative to source directory.");
                }
            } else {
                log::debug!("Skipping root directory relative to source directory.");
            }
        } else {
            log::warn!("Failed to strip prefix from path, path is outside of source directory: {:?}", path);
        }
    }

    // Finish the tar, this will finalize the tar.gz file
    tar_builder.into_inner()?.finish()?;
    log::info!("Successfully created tar.gz file: {:?}", output_file.as_ref());
    Ok(())
}

fn get_media(metadata: Metadata, filename: String, server_type: &ServerType) -> Media {
    let mut media = Media::new(format!("KiwiTech/{}/{}", server_type.to_string().to_uppercase(), filename.as_str()));
    media.content_type = Cow::from("application/gzip");
    media.content_length = Some(metadata.len());

    media
}