mod api;
mod cookies;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    // TODO: pass by CLI
    let cookie_data = include_str!("cookies.json");
    let user = "cN3rd";

    // build app context
    let api_context: api::BandcampAPIContext = api::BandcampAPIContext::new(user, cookie_data);

    // higher-level fetching of things, todo

    // let fan_page_blob = api_context.get_initial_fan_page_data_text().await?;
    // if fan_page_blob.collection_data.redownload_urls.is_none()
    //     || (fan_page_blob
    //         .collection_data
    //         .redownload_urls
    //         .as_ref()
    //         .unwrap()
    //         .is_empty())
    // {
    //     println!("No download links could by found in the collection page. This can be caused by an outdated or invalid cookies file.");
    // }

    // // download visible things
    // let mut downloads = fan_page_blob.collection_data.redownload_urls.unwrap();

    // downloads.extend(
    //     api_context
    //         .retrieve_download_urls(
    //             fan_page_blob.fan_data.fan_id,
    //             fan_page_blob.collection_data.last_token.unwrap().as_str(),
    //             "collection_items",
    //         )
    //         .await?
    //         .into_iter(),
    // );

    // // download hidden (TODO)
    // println!("{:?}", downloads);

    let _sale_id = "r178743155";
    let url = "https://bandcamp.com/download?from=collection&payment_id=1170963116&sig=bf60707c02a8f358afa01f3cd3e020c7&sitem_id=178743155";

    let bandcamp_data = api_context.retrieve_digital_download_item_data(url).await?;
    let _link = api_context
        .retrieve_digital_download_link(&bandcamp_data, "flac")
        .await?;

    Ok(())
}
