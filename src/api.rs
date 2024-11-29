use regex_lite::Regex;
use reqwest::Client;
use reqwest_cookie_store::CookieStoreMutex;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime},
};

use crate::{
    error::{
        ContextCreationError, DigitalDownloadError, InformationRetrievalError,
        ReleaseRetrievalError,
    },
    middlewares::{RateLimitMiddleware, RetryMiddleware},
};

pub mod data;

fn stat_response_regex() -> &'static Regex {
    static STAT_DOWNLOAD_REGEX: OnceLock<Regex> = OnceLock::new();
    STAT_DOWNLOAD_REGEX.get_or_init(|| {
        Regex::new(
            r"if\s*\(\s*window\.Downloads\s*\)\s*\{\s*Downloads\.statResult\s*\(\s*(.*)\s*\)\s*};",
        )
        .expect("Regex pattern for \"stat_response_regex\" should compile successfully")
    })
}

fn data_blob_regex() -> &'static Regex {
    static DATA_BLOB_REGEX: OnceLock<Regex> = OnceLock::new();
    DATA_BLOB_REGEX.get_or_init(|| {
        Regex::new(r#"(?s)<div\s+(?:[^>]*?\s+)?id="pagedata"(?:\s+[^>]*?)?\s+data-blob="((?:[^"\\]|\\.)*)""#)
            .expect("Regex pattern for \"data_blob_regex\" should compile successfully")
    })
}

fn generate_token(item_id: i64, item_type: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    format!("{timestamp}:{item_id}:{item_type}::")
}

pub struct BandcampAPIContext {
    pub client: ClientWithMiddleware,
}

pub type SaleIdUrlMap = HashMap<String, String>;

impl BandcampAPIContext {
    pub fn new(cookie_data: &str) -> Result<Self, ContextCreationError> {
        let cookie_store = crate::cookies::read_json_file(cookie_data, "https://bandcamp.com")?;
        let client = Client::builder()
            .cookie_provider(Arc::new(CookieStoreMutex::new(cookie_store)))
            .build()?;

        let client = ClientBuilder::new(client)
            .with(RetryMiddleware::new(5))
            .with(RateLimitMiddleware::new(10, Duration::from_secs(10)))
            .build();

        Ok(Self { client })
    }

    pub async fn get_summary(
        &self,
    ) -> Result<data::ParsedFanCollectionSummary, InformationRetrievalError> {
        let response = self
            .client
            .get("https://bandcamp.com/api/fan/2/collection_summary")
            .send()
            .await?;
        let response_text = response.text().await?;
        let parsed_summary =
            serde_json::from_str::<data::ParsedFanCollectionSummary>(&response_text)?;

        Ok(parsed_summary)
    }

    pub async fn get_all_releases(
        &self,
        summary: &data::ParsedFanCollectionSummary,
        include_hidden: bool,
    ) -> Result<SaleIdUrlMap, ReleaseRetrievalError> {
        let mut collection = SaleIdUrlMap::new();

        let first_item = summary
            .collection_summary
            .tralbum_lookup
            .as_ref()
            .unwrap()
            .iter()
            .next()
            .unwrap();

        let token = generate_token(first_item.1.item_id, &first_item.1.item_type);

        collection.extend(
            self.get_webui_download_urls(summary.fan_id, &token, "collection_items")
                .await?,
        );

        if include_hidden {
            collection.extend(
                self.get_webui_download_urls(summary.fan_id, &token, "hidden_items")
                    .await?,
            );
        }
        Ok(collection)
    }

    pub async fn get_webui_download_urls(
        &self,
        fan_id: i64,
        last_token: &str,
        collection_name: &str,
    ) -> Result<SaleIdUrlMap, ReleaseRetrievalError> {
        let mut download_urls = SaleIdUrlMap::new();
        let mut current_token = last_token.to_string();

        loop {
            let body = format!(
                "{{\"fan_id\": {fan_id}, \"older_than_token\": \"{current_token}\", \"count\":100000}}"
            );

            let response = self
                .client
                .post(format!(
                    "https://bandcamp.com/api/fancollection/1/{collection_name}"
                ))
                .body(body)
                .send()
                .await?;

            let parsed_collection_data: data::ParsedCollectionItems =
                serde_json::from_str(&response.text().await?)?;

            let Some(redownload_urls) = parsed_collection_data.redownload_urls else {
                break;
            };

            download_urls.extend(redownload_urls);

            if !parsed_collection_data.more_available {
                break;
            }
            current_token = parsed_collection_data
                .last_token
                .expect("Server returned more_available=true but no last_token");
        }

        Ok(download_urls)
    }

    pub async fn get_digital_download_item(
        &self,
        item_url: &str,
    ) -> Result<Option<data::DigitalItem>, InformationRetrievalError> {
        let response = self.client.get(item_url).send().await?;
        let response_data = response.text().await?;

        let data_blob = data_blob_regex()
            .captures(&response_data)
            .ok_or(InformationRetrievalError::DataBlobNotFound)?
            .get(1)
            .ok_or(InformationRetrievalError::DataBlobNotFound)?
            .as_str();
        let data_blob = htmlize::unescape(data_blob);

        let bandcamp_data = serde_json::from_str::<data::ParsedBandcampData>(&data_blob)?;
        if bandcamp_data.digital_items.is_empty() {
            return Ok(None);
        }

        Ok(Some(bandcamp_data.digital_items[0].clone()))
    }

    pub async fn get_digital_download_link(
        &self,
        digital_item: &data::DigitalItem,
        download_format: data::DownloadFormat,
    ) -> Result<String, DigitalDownloadError> {
        self.qualify_digital_download_link(get_unqualified_digital_download_link(
            digital_item,
            download_format,
        )?)
        .await
    }

    pub async fn qualify_digital_download_link(
        &self,
        download_link: &str,
    ) -> Result<String, DigitalDownloadError> {
        let mut actual_dl_link = download_link.to_string();
        loop {
            let inner = self
                .retrieve_digital_download_stat_data(&actual_dl_link)
                .await?;

            match get_qualified_digital_download_url(&inner) {
                Ok(url) => return Ok(url),
                Err(DigitalDownloadError::JsonResponseErrorCode(url)) => {
                    actual_dl_link = url;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn retrieve_digital_download_stat_data(
        &self,
        download_link: &str,
    ) -> Result<String, DigitalDownloadError> {
        let stat_download_url = download_link
            .replace("/download/", "/statdownload/")
            .replace("http://", "https://")
            + "&.vrs=1"
            + "&.rand="
            + &fastrand::i32(..).to_string();
        let stat_download_response: reqwest::Response =
            self.client.get(stat_download_url).send().await?;
        let stat_download_response_body = stat_download_response.text().await?;

        Ok(stat_download_response_body)
    }
}

pub fn get_unqualified_digital_download_link(
    digital_item: &data::DigitalItem,
    download_format: data::DownloadFormat,
) -> Result<&str, DigitalDownloadError> {
    let digital_download_list = digital_item
        .downloads
        .as_ref()
        .ok_or(DigitalDownloadError::NoDownloadLinksFound)?;

    if digital_download_list.is_empty() {
        return Err(DigitalDownloadError::NoDownloadLinksFound);
    }

    Ok(&digital_download_list
        .get(&download_format)
        .ok_or(DigitalDownloadError::RequestedFormatLinkNotFound)?
        .url)
}

pub fn get_qualified_digital_download_url(
    stat_response_body: &str,
) -> Result<String, DigitalDownloadError> {
    let inner_json = stat_response_regex()
        .captures(stat_response_body)
        .ok_or(DigitalDownloadError::JsonBodyNotFound)?
        .get(1)
        .ok_or(DigitalDownloadError::JsonBodyNotFound)?
        .as_str();

    let inner_data: data::ParsedStatDownload = serde_json::from_str(inner_json)?;
    if Some("err".into()) == inner_data.result {
        return Err(DigitalDownloadError::JsonResponseErrorCode(format!(
            "https://{}",
            inner_data.url
        )));
    }

    inner_data
        .download_url
        .ok_or(DigitalDownloadError::NoLinkFound)
}
