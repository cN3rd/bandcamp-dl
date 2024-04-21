#![deny(unsafe_code)]
#![warn(
    clippy::cognitive_complexity,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::float_equality_without_abs,
    keyword_idents,
    clippy::missing_const_for_fn,
    missing_copy_implementations,
    missing_debug_implementations,
    // clippy::missing_docs_in_private_items,
    // clippy::missing_errors_doc,
    // clippy::missing_panics_doc,
    clippy::mod_module_files,
    non_ascii_idents,
    noop_method_call,
    clippy::option_if_let_else,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::semicolon_if_nothing_returned,
    clippy::unseparated_literal_suffix,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::suspicious_operation_groupings,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    clippy::unused_self,
    clippy::use_debug,
    clippy::used_underscore_binding,
    clippy::useless_let_if_seq,
    clippy::wildcard_dependencies,
    clippy::wildcard_imports
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

    println!("Retrieving all releases...");
    let releases = api_context.get_all_releases(&fanpage_data, false).await?;

    // finding releases not found in regular scopes
    let mut digital_item_tasks = JoinSet::new();
    for (key, item_url) in &releases {
        if !download_cache.contains_key(key) {
            let api_context = Arc::clone(&api_context);

            // Clone `item_url` and `key` for use in the async block
            let item_url = item_url.clone();
            let key = key.clone();

            digital_item_tasks.spawn(async move {
                let result = api_context.get_digital_download_item(&item_url).await;
                (result, key)
            });
        }
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
        );
    }

    // TODO: pretend download

    Ok(())
}
