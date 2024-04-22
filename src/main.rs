// Safety lints
#![deny(bare_trait_objects)]
#![deny(clippy::as_ptr_cast_mut)]
#![deny(clippy::large_stack_arrays)]
// Performance lints
#![warn(clippy::inefficient_to_string)]
#![warn(clippy::invalid_upcast_comparisons)]
#![warn(clippy::iter_with_drain)]
#![warn(clippy::linkedlist)]
#![warn(clippy::mutex_integer)]
#![warn(clippy::naive_bytecount)]
#![warn(clippy::needless_bitwise_bool)]
#![warn(clippy::needless_collect)]
#![warn(clippy::or_fun_call)]
#![warn(clippy::stable_sort_primitive)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::trivial_regex)]
#![warn(clippy::trivially_copy_pass_by_ref)]
#![warn(clippy::unnecessary_join)]
#![warn(clippy::unused_async)]
#![warn(clippy::zero_sized_map_values)]
// Correctness lints
#![deny(clippy::case_sensitive_file_extension_comparisons)]
#![deny(clippy::copy_iterator)]
#![deny(clippy::expl_impl_clone_on_copy)]
#![deny(clippy::float_cmp)]
#![warn(clippy::imprecise_flops)]
#![deny(clippy::manual_instant_elapsed)]
#![deny(clippy::mem_forget)]
#![deny(clippy::path_buf_push_overwrite)]
#![deny(clippy::same_functions_in_if_condition)]
#![deny(clippy::unchecked_duration_subtraction)]
#![deny(clippy::unicode_not_nfc)]
// Clarity/formatting lints
#![warn(clippy::checked_conversions)]
#![allow(clippy::comparison_chain)]
#![warn(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::enum_variant_names)]
#![warn(clippy::explicit_deref_methods)]
#![warn(clippy::filter_map_next)]
#![warn(clippy::flat_map_option)]
#![warn(clippy::fn_params_excessive_bools)]
#![warn(clippy::implicit_clone)]
#![warn(clippy::iter_not_returning_iterator)]
#![warn(clippy::iter_on_empty_collections)]
#![warn(clippy::macro_use_imports)]
#![warn(clippy::manual_clamp)]
#![warn(clippy::manual_let_else)]
#![warn(clippy::manual_ok_or)]
#![warn(clippy::manual_string_new)]
#![warn(clippy::map_flatten)]
#![warn(clippy::match_bool)]
#![warn(clippy::mut_mut)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::needless_continue)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![warn(clippy::range_minus_one)]
#![warn(clippy::range_plus_one)]
#![warn(clippy::ref_binding_to_reference)]
#![warn(clippy::ref_option_ref)]
#![warn(clippy::trait_duplication_in_bounds)]
#![warn(clippy::unused_peekable)]
#![warn(clippy::unused_rounding)]
#![warn(clippy::unused_self)]
#![allow(clippy::upper_case_acronyms)]
#![warn(clippy::verbose_bit_mask)]
#![warn(clippy::verbose_file_reads)]
// Documentation lints
#![warn(clippy::doc_link_with_quotes)]
#![warn(clippy::doc_markdown)]
#![warn(clippy::missing_errors_doc)]
#![warn(clippy::missing_panics_doc)]
// FIXME: We should fix instances of this lint and change it to `warn`
#![allow(clippy::missing_safety_doc)]

use std::{collections::HashMap, sync::Arc};

use tokio::task::JoinSet;

mod api;
mod cache;
mod cookies;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        let (result, key) = result?;
        if let Some(item_data) = result? {
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
