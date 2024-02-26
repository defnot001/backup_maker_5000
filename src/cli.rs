use clap::{Parser, ValueEnum};
use std::fmt::Display;

#[derive(Debug, Parser)]
#[command(
name = "pterobackup",
version = "1.0",
about = "A simple tool to take a backup on a pterodactyl server and then upload it to a gcs bucket.",
long_about = None,
)]
pub struct Args {
    /// Which server to backup
    #[arg(value_enum)]
    pub server: ServerType,

    /// Verbose mode (-v, --verbose)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Name of the folder to exclude from the backup
    #[arg(short = 'e', long = "exclude")]
    pub exclude: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ServerType {
    /// Survival Server
    Smp,
    /// Creative Server
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
