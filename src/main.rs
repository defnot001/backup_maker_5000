#![allow(unused, dead_code)]

use clap::{Parser, ValueEnum};
use serde::Deserialize;
use google_cloud_storage::client::{ClientConfig, Client as GCSClient};
use google_cloud_storage::client::google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::http::buckets::list::ListBucketsRequest;
use reqwest::{Client as ReqwestClient, ClientBuilder};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::header;
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

#[derive(Debug, Deserialize)]
struct Config {
    ptero_url: String,
    ptero_key: String,
    gcs_credentials: String,
    mc_server_host: String,
    servers: Servers,
}

#[derive(Debug, Deserialize)]
struct Servers {
    smp: MCServer,
    cmp: MCServer,
}

#[derive(Debug, Deserialize)]
struct MCServer {
    server_id: String,
    port: u16,
}

impl Config {
    fn load() -> anyhow::Result<Self> {
        let config_string = std::fs::read_to_string("config.json")?;
        serde_json::from_str(&config_string).map_err(Into::into)
    }
}

#[derive(Debug, Deserialize)]
struct Backup {
    pub uuid: Uuid,
    pub name: String,
    pub ignored_files: Vec<String>,
    pub checksum: Option<String>,
    pub bytes: u64,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub is_locked: bool,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = Config::load().expect("Failed to load config");
    let base_url = format!("{}/api/client", &config.ptero_url);

    let gcs_client = generate_gcs_client(config.gcs_credentials.as_str()).await.expect("Failed to generate GCS Client");
    let reqwest_client = generate_reqwest_client(config.ptero_key.as_str()).expect("Failed to generate Reqwest Client");

    let created_backup = create_backup(&reqwest_client, &args.server, &config.servers, base_url.as_str()).await.expect("Failed to create backup.");
}

async fn generate_gcs_client(credentials_file_path: &str) -> anyhow::Result<GCSClient> {
    let credentials_file = CredentialsFile::new_from_file(credentials_file_path.to_string()).await?;
    let gcs_config = ClientConfig::default().with_credentials(credentials_file).await?;
    Ok(GCSClient::new(gcs_config))
}

fn generate_reqwest_client(ptero_key: &str) -> anyhow::Result<ReqwestClient> {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", HeaderValue::from_static("application/json"));
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers.insert("Authorization", HeaderValue::from_str(format!("Bearer {}", ptero_key).as_str()).unwrap());

    Ok(ClientBuilder::new()
        .default_headers(headers)
        .build()?)
}

async fn create_backup(req_client: &ReqwestClient, server: &ServerType, servers: &Servers, base_url: &str) -> anyhow::Result<Backup> {
    let server_id = get_server_id(server, servers);
    let req_url = format!("{}/api/client/servers/{}/backups",base_url, server_id);

    Ok(req_client.post(req_url).send().await?.json::<Backup>().await?)
}

async fn wait_until_backup_done() {
    
}

fn get_server_id(server:&ServerType, servers: &Servers) -> String {
    let server_id = match server {
        ServerType::Smp => &servers.smp.server_id,
        ServerType::Cmp => &servers.cmp.server_id,
    };

    server_id.to_string()
}