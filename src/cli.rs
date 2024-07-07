use std::{collections::HashMap, sync::Arc};

use tokio::task::JoinSet;
use trauma::{download::Download, downloader::DownloaderBuilder};

use crate::{api, cache};
use clap::Parser;

#[derive(Parser, PartialEq, Eq)]
#[command(name = "bandcamp-dl")]
pub struct Cli {
    #[arg(help = "The Bandcamp user account from which all releases must be downloaded.")]
    user: String,

    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    #[arg(help = "Cookie file to read, in the format of \"cookies.txt\".")]
    cookie_file: std::path::PathBuf,

    #[arg(long)]
    #[arg(help = "Don't download hidden items in the collection.")]
    skip_hidden: bool,

    #[arg(long, value_enum, default_value_t = api::DownloadFormat::Flac)]
    #[arg(help = "The audio format requested for newly downloaded audio.")]
    audio_format: api::DownloadFormat,

    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    #[arg(
        help = "Folder to download files to. If no value is given, defaults to the current directory."
    )]
    download_folder: Option<std::path::PathBuf>,

    #[arg(long)]
    #[arg(help = "Fetch information correctly but don't actually download.")]
    dry_run: bool,
}

pub async fn run_program(cli: Cli) -> anyhow::Result<()> {
    println!("Parsing download cache...");
    let download_cache_data = include_str!("data/bandcamp-collection-downloader.cache");
    let download_cache = cache::read_download_cache(download_cache_data)?;

    // build app context
    let cookie_data = std::fs::read_to_string(cli.cookie_file)?;
    let api_context = Arc::new(api::BandcampAPIContext::new(&cli.user, &cookie_data)?);

    println!("Retrieving Bandcamp Fan Page Data...");
    let fanpage_data = api_context.get_fanpage_data().await?;

    println!("Retrieving all releases...");
    let releases = api_context
        .get_all_releases(&fanpage_data, !cli.skip_hidden)
        .await?;

    // finding releases not found in regular scopes
    println!("Finding new releases...");
    let items_to_download = find_new_releases(releases, download_cache, &api_context).await?;

    // fetch all download links
    println!("Fetching releases in {}...", cli.audio_format);

    let mut retrieve_download_links_tasks = JoinSet::new();
    for (key, digital_item) in items_to_download {
        let api_context = Arc::clone(&api_context);
        retrieve_download_links_tasks.spawn(async move {
            let result = api_context
                .get_digital_download_link(&digital_item, cli.audio_format)
                .await;
            (result, digital_item, key)
        });
    }

    let mut downloads = Vec::new();

    while let Some(result) = retrieve_download_links_tasks.join_next().await {
        let (result, digital_item, key) = result?;
        let url = result?;

        downloads.push(Download::try_from(url.as_str()).unwrap());

        println!(
            "Download link for \"{}\" by {} ({}): {}",
            digital_item.title, digital_item.artist, key, url
        );
    }

    if !cli.dry_run {
        let downloader = DownloaderBuilder::new()
            .directory(cli.download_folder.unwrap())
            .build();

        downloader.download(&downloads).await;
    }

    Ok(())
}

async fn find_new_releases(
    releases: api::SaleIdUrlMap,
    download_cache: cache::DownloadCache,
    api_context: &Arc<api::BandcampAPIContext>,
) -> Result<HashMap<String, api::DigitalItem>, anyhow::Error> {
    let mut digital_item_tasks = JoinSet::new();
    for (key, item_url) in &releases {
        if !download_cache.contains_key(key) {
            let api_context_clone = Arc::clone(api_context);

            // Clone `item_url` and `key` for use in the async block
            let item_url_clone = item_url.clone();
            let key_clone = key.clone();

            digital_item_tasks.spawn(async move {
                let result = api_context_clone
                    .get_digital_download_item(&item_url_clone)
                    .await;
                (result, key_clone)
            });
        }
    }

    let mut items_to_download = HashMap::new();
    while let Some(task_result) = digital_item_tasks.join_next().await {
        let (digital_item_result, key) = task_result?;
        if let Some(item_data) = digital_item_result? {
            println!(
                "New item: \"{}\" by \"{}\" ({})",
                item_data.title, item_data.artist, key
            );
            items_to_download.insert(key, item_data);
        }
    }

    Ok(items_to_download)
}
