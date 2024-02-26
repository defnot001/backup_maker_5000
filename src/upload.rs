use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures::Stream;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Body, Client};
use tokio::fs::File;
use futures::stream::TryStreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use crate::cli::ServerType;

struct ProgressStream<S> {
    inner: S,
    progress_bar: ProgressBar,
}

impl<S, E> Stream for ProgressStream<S>
    where
        S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    type Item = Result<Bytes, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.progress_bar.inc(chunk.len() as u64);
                Poll::Ready(Some(Ok(chunk)))
            },
            other => other,
        }
    }
}

pub async fn upload_file(server_name: &str, server_type: &ServerType, bucket_name: &str, object_name: &str, access_token: &str) -> anyhow::Result<()> {
    let file = File::open(object_name).await?;
    let client = Client::new();
    let progress_bar = get_progress_bar(file.metadata().await?.len())?;
    log::debug!("Progress bar created.");
    log::info!("Uploading file to GCS...");

    let url = format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
         bucket_name, get_upload_path(object_name, server_type, server_name)
    );
    log::debug!("Upload URL: {}", url);

    let stream = FramedRead::new(file, BytesCodec::new())
        .map_ok(|bytes| bytes.freeze());
    log::debug!("Stream created.");


    let progress_stream = ProgressStream {
        inner: stream,
        progress_bar,
    };

    let body = Body::wrap_stream(progress_stream);

    client.post(&url)
        .bearer_auth(access_token)
        .header("Content-Type", "application/gzip")
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    log::info!("File uploaded successfully.");
    Ok(())
}

fn get_upload_path(file_name: &str, server_type: &ServerType, server_name: &str) -> String {
    format!("{}/{}/{}", server_name, server_type.to_string().to_uppercase(), file_name)
}

fn get_progress_bar(file_size: u64) -> anyhow::Result<ProgressBar> {
    let progress_bar = ProgressBar::new(file_size);
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));

    Ok(progress_bar)
}