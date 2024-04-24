use std::{collections::HashMap, sync::Arc};

use tokio::task::JoinSet;

use crate::{api, cache};

pub(crate) async fn main() -> anyhow::Result<()> {
    // TODO: pass by CLI
    let cookie_data = include_str!("data/cookies.json");
    let user = "cN3rd";

    println!("Parsing download cache...");
    let download_cache_data = include_str!("data/bandcamp-collection-downloader.cache");
    let download_cache = cache::read_download_cache(download_cache_data)?;

    // build app context
    let api_context = Arc::new(api::BandcampAPIContext::new(user, cookie_data)?);

    println!("Retrieving Bandcamp Fan Page Data...");
    let fanpage_data = api_context.get_fanpage_data().await?;

    println!("Retrieving all releases...");
    let releases = api_context.get_all_releases(&fanpage_data, false).await?;

    // finding releases not found in regular scopes
    let mut digital_item_tasks = JoinSet::new();
    for (key, item_url) in &releases {
        if !download_cache.contains_key(key) {
            let api_context_clone = Arc::clone(&api_context);

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
                "Not found: \"{}\" by \"{}\" ({})",
                item_data.title, item_data.artist, key
            );
            items_to_download.insert(key, item_data);
        }
    }

    // fetch all download links
    let download_format = api::DownloadFormat::FLAC;
    let mut retrieve_download_links_tasks = JoinSet::new();
    for (key, digital_item) in items_to_download {
        let api_context = Arc::clone(&api_context);
        retrieve_download_links_tasks.spawn(async move {
            let result = api_context
                .get_digital_download_link(&digital_item, download_format)
                .await;
            (result, digital_item, key)
        });
    }

    while let Some(result) = retrieve_download_links_tasks.join_next().await {
        let (result, digital_item, key) = result?;
        let url = result?;

        println!(
            "Download link for \"{}\" by {} ({}): {}",
            digital_item.title, digital_item.artist, key, url
        );
    }

    // TODO: pretend download

    Ok(())
}
