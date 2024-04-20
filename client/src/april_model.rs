use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::warn;
use std::fs::File;
use std::io::{stdin, BufRead, Write};
use std::path::Path;
use url::Url;
pub async fn download(path: &Path) -> Result<()> {
    warn!("speech recognition model does not exist in data directory: prompting for download");
    println!("Choose a model to download:");
    println!("1. Low precision model that works with poor quality audio input");
    println!("2. High precision model that requires good quality audio input");
    println!("default: 2");

    let url = match stdin().lock().lines().next() {
        Some(Ok(s)) if s == "1" => Url::parse("https://april.sapples.net/aprilv0_en-us.april")?,
        _ => Url::parse("https://april.sapples.net/april-english-dev-01110_en.april")?,
    };

    // Create a progress bar with the expected file size
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let content_length = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(content_length);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("Of."),
    );

    // Create a temporary file to save the downloaded content
    let mut file = File::create(&path)?;

    // Download the file in chunks and update the progress bar
    let mut downloaded = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        file.write_all(&data)?;
        downloaded += data.len() as u64;
        pb.set_position(downloaded);
    }

    // Finish the progress bar
    pb.finish_with_message("Download complete!");
    Ok(())
}
