use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::warn;
use soup::prelude::*;
use std::fs::{self, File};
use std::io::copy;
use std::io::{stdin, BufRead, Write};
use std::path::{Path, PathBuf};
use url::Url;
pub async fn download(path: &Path) -> Result<()> {
    warn!("speech recognition model does not exist in data directory: prompting for download");
    println!("Choose a model to download:");
    let listing = reqwest::get("https://alphacephei.com/vosk/models")
        .await?
        .text()
        .await?;
    let soup = Soup::new(&listing);
    let links: Vec<_> = soup
        .tag("a")
        .find_all()
        .filter_map(|link| link.get("href"))
        .filter(|href| href.starts_with("https://alphacephei.com/vosk/models"))
        .collect();
    for (i, link) in links.iter().enumerate() {
        println!("{}. {}", i + 1, link);
    }
    println!("default: 1");

    let url = match stdin()
        .lock()
        .lines()
        .next()
        .and_then(|res| res.ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(i) => Url::parse(&links[i - 1])?,
        _ => Url::parse(&links[0])?,
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

    let dest = &path.with_extension("zip");
    if !dest.exists() {
        let mut file = File::create(dest)?;

        // Download the file in chunks and update the progress bar
        let mut downloaded = 0;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let data = chunk?;
            file.write_all(&data)?;
            downloaded += data.len() as u64;
            pb.set_position(downloaded);
        }
    }

    let file = File::open(&dest)?;

    // Create a ZipArchive from the file.
    let mut archive = zip::ZipArchive::new(file)?;

    // Iterate over each file in the archive.
    for i in 0..archive.len() {
        // Get the file at the current index.
        let mut file = archive.by_index(i)?;

        // Get the path to extract the file to.
        let outpath = match file.enclosed_name() {
            Some(p) => path.join(PathBuf::from_iter(p.components().skip(1))),
            None => continue, // Skip to the next file if the path is None.
        };

        // Check if the file is a directory.
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?; // Create the directory.
        } else {
            // Create parent directories if they don't exist.
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }

            // Create and copy the file contents to the output path.
            let mut outfile = File::create(&outpath)?;
            copy(&mut file, &mut outfile)?;
        }

        // Set file permissions if running on a Unix-like system.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    pb.finish_with_message("Download complete!");

    Ok(())
}
