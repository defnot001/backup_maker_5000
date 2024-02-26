use crate::cli::ServerType;
use crate::Config;
use flate2::write::GzEncoder;
use std::fs::OpenOptions;
use std::path::Path;
use walkdir::WalkDir;

pub fn compress_volume(
    server_type: &ServerType,
    exclude: &Option<String>,
    config: &Config,
    file_name: &str,
) -> anyhow::Result<()> {
    let volume_path = match server_type {
        ServerType::Smp => format!("{}/{}", config.volumes_path, config.smp_uuid),
        ServerType::Cmp => format!("{}/{}", config.volumes_path, config.cmp_uuid),
    };

    create_tar_gz(volume_path, file_name.to_string(), exclude)?;
    Ok(())
}

pub fn generate_file_name(server_name: &str, server_type: &ServerType) -> String {
    let now = chrono::Utc::now();
    format!(
        "{}_{}_{}.tar.gz",
        now.format("%Y-%m-%d"),
        server_name,
        server_type.to_string().to_uppercase()
    )
}

fn create_tar_gz<P: AsRef<Path>>(
    source_dir: P,
    output_file: P,
    exclude: &Option<String>,
) -> anyhow::Result<()> {
    log::info!("Creating tar.gz file: {:?}", output_file.as_ref());
    let tar_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_file.as_ref())?;
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
            log::warn!(
                "Failed to strip prefix from path, path is outside of source directory: {:?}",
                path
            );
        }
    }

    // Finish the tar, this will finalize the tar.gz file
    tar_builder.into_inner()?.finish()?;
    log::info!(
        "Successfully created tar.gz file: {:?}",
        output_file.as_ref()
    );
    Ok(())
}
