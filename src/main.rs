#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

use std::{collections::HashMap, sync::Arc};

use tokio::task::JoinSet;

mod api;
mod cache;
mod cookies;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    // TODO: pass by CLI
    let cookie_data = include_str!("data/cookies.json");
    let user = "cN3rd";

    println!("Parsing download cache...");
    let download_cache_data = include_str!("data/bandcamp-collection-downloader.cache");
    let download_cache = cache::read_download_cache(download_cache_data);

    // build app context
    let api_context = Arc::new(api::BandcampAPIContext::new(user, cookie_data));

    println!("Retrieving Bandcamp Fan Page Data...");
    let fanpage_data = api_context.get_fanpage_data().await?;

    println!("Retreiving all releases...");
    let releases = api_context.get_all_releases(&fanpage_data, false).await?;

    // finding releases not found in regular scopes
    let mut digital_item_tasks = JoinSet::new();
    for (&ref key, &ref item_url) in releases
        .iter()
        .filter(|&(key, _)| !download_cache.contains_key(key))
    {
        let item_url = item_url.clone();
        let key = key.clone();
        let api_context = Arc::clone(&api_context);

        digital_item_tasks.spawn(async move {
            let result = api_context.get_digital_download_item(&item_url).await;
            (result, key)
        });
    }

    let mut items_to_download = HashMap::new();
    while let Some(result) = digital_item_tasks.join_next().await {
        let (result, key) = result.unwrap();
        if let Some(item_data) = result.unwrap() {
            println!(
                "Not found: \"{}\" by \"{}\" ({})",
                item_data.title, item_data.artist, key
            );
            items_to_download.insert(key, item_data);
        }
    }

    // fetch all download links
    let download_format = "flac";
    let mut retrieve_download_links_tasks = JoinSet::new();
    for (key, digital_item) in items_to_download {
        let api_context = Arc::clone(&api_context);
        retrieve_download_links_tasks.spawn(async move {
            let digital_item = digital_item;
            let result = api_context
                .get_digital_download_link(&digital_item, download_format)
                .await;
            (result, digital_item, key)
        });
    }

    while let Some(result) = retrieve_download_links_tasks.join_next().await {
        let (result, digital_item, key) = result.unwrap();
        let url = result?;

        println!(
            "Download link for \"{}\" by {} ({}): {}",
            digital_item.title, digital_item.artist, key, url
        )
    }

    // TODO: pretend download

    Ok(())
}
