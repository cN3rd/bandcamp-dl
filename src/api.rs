use regex::Regex;
use reqwest::Client;
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

#[derive(Serialize, Deserialize)]
pub struct ParsedFanpageData {
    pub fan_data: FanData,
    pub collection_data: CollectionData,
    pub hidden_data: CollectionData,
    pub item_cache: ItemCache,
}

#[derive(Serialize, Deserialize)]
pub struct FanData {
    pub fan_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ItemCache {
    pub collection: HashMap<String, CachedItem>,
    pub hidden: HashMap<String, CachedItem>,
}

#[derive(Serialize, Deserialize)]
pub struct CachedItem {
    pub sale_item_id: i64,
    pub band_name: String,
    pub item_title: String,
}

#[derive(Serialize, Deserialize)]
pub struct CollectionData {
    pub batch_size: i64,
    pub item_count: i64,
    pub last_token: Option<String>,
    pub redownload_urls: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct ParsedCollectionItems {
    pub more_available: bool,
    pub last_token: String,
    pub redownload_urls: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct ParsedBandcampData {
    pub digital_items: Vec<DigitalItem>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadData {
    pub size_mb: String,
    pub description: String,
    pub encoding_name: String,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DigitalItem {
    pub downloads: Option<HashMap<String, DownloadData>>,
    pub package_release_date: Option<String>,
    pub title: String,
    pub artist: String,
    pub download_type: String,
    pub download_type_str: String,
    pub item_type: String,
    pub art_id: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ParsedStatDownload {
    pub download_url: Option<String>,
    pub url: String,
}

pub struct BandcampAPIContext {
    pub client: Client,
    pub user_name: String,
}

fn stat_response_regex() -> &'static Regex {
    static STAT_DOWNLOAD_REGEX: OnceLock<Regex> = OnceLock::new();
    STAT_DOWNLOAD_REGEX.get_or_init(|| {
        Regex::new(
            r"if\s*\(\s*window\.Downloads\s*\)\s*\{\s*Downloads\.statResult\s*\(\s*(.*)\s*\)\s*};",
        )
        .unwrap()
    })
}

impl BandcampAPIContext {
    pub fn new(user: &str, cookie_data: &str) -> Self {
        let cookie_store: cookie_store::CookieStore =
            crate::cookies::read_json_file(cookie_data, "https://bandcamp.com");
        let cookie_store_mutex = CookieStoreMutex::new(cookie_store);
        let client = Client::builder()
            .cookie_provider(Arc::new(cookie_store_mutex))
            .build()
            .unwrap();

        Self {
            client,
            user_name: user.to_owned(),
        }
    }

    pub async fn get_fanpage_data(&self) -> Result<ParsedFanpageData, reqwest::Error> {
        let res = self
            .client
            .get(format!("https://bandcamp.com/{}", self.user_name))
            .send()
            .await?;

        let html = res.text().await?;
        let html = scraper::Html::parse_document(html.as_str());
        let selector = scraper::Selector::parse("#pagedata").unwrap();
        let selection = html.select(&selector).next().unwrap();
        let attr = selection.attr("data-blob").unwrap();

        let parsed_fanpage_data: ParsedFanpageData = serde_json::from_str(attr).unwrap();

        Ok(parsed_fanpage_data)
    }

    pub async fn get_all_releases(
        &self,
        fanpage_data: &ParsedFanpageData,
        include_hidden: bool,
    ) -> Result<HashMap<String, String>, reqwest::Error> {
        if fanpage_data.collection_data.redownload_urls.is_none()
            || (fanpage_data
                .collection_data
                .redownload_urls
                .as_ref()
                .unwrap()
                .is_empty())
        {
            println!("No download links could by found in the collection page. This can be caused by an outdated or invalid cookies file.");
        }

        // download visible things
        let mut all_downloads = fanpage_data
            .collection_data
            .redownload_urls
            .clone()
            .unwrap();

        if !include_hidden {
            let hidden_items = &fanpage_data.item_cache.hidden;
            all_downloads = all_downloads
                .into_iter()
                .filter(|(k, _)| !hidden_items.contains_key(k))
                .collect::<HashMap<_, _>>(); // TODO: fix this
        }

        // Get the rest of the non-hidden collection
        if fanpage_data.collection_data.item_count > fanpage_data.collection_data.batch_size {
            let last_token = fanpage_data.collection_data.last_token.clone().unwrap();
            let fan_id = fanpage_data.fan_data.fan_id;
            all_downloads.extend(
                self.get_webui_download_urls(fan_id, &last_token, "collection_items")
                    .await?
                    .into_iter(),
            );

            if include_hidden {
                let last_token = fanpage_data.hidden_data.last_token.clone().unwrap();
                all_downloads.extend(
                    self.get_webui_download_urls(fan_id, &last_token, "hidden_items")
                        .await?
                        .into_iter(),
                );
            }
        }

        Ok(all_downloads)
    }

    pub async fn get_webui_download_urls(
        &self,
        fan_id: i64,
        last_token: &str,
        collection_name: &str,
    ) -> Result<HashMap<String, String>, reqwest::Error> {
        let mut more_available = true;
        let mut last_token = last_token.to_owned();
        let mut download_urls: HashMap<String, String> = HashMap::new();

        while more_available {
            let response = self
                .client
                .post(format!(
                    "https://bandcamp.com/api/fancollection/1/{collection_name}"
                ))
                .body(format!(
                    "{{\"fan_id\": {fan_id}, \"older_than_token\": \"{last_token}\"}}"
                ))
                .send()
                .await?;
            let response_data = response.text().await?;
            let parsed_collection_data: ParsedCollectionItems =
                serde_json::from_str(&response_data).unwrap();

            download_urls.extend(parsed_collection_data.redownload_urls.into_iter());

            more_available = parsed_collection_data.more_available;
            last_token = parsed_collection_data.last_token;
        }
        Ok(download_urls)
    }

    pub async fn get_digital_download_item(
        &self,
        item_url: &str,
    ) -> Result<Option<DigitalItem>, reqwest::Error> {
        let response = self.client.get(item_url).send().await?;
        let response_data = response.text().await?;

        let html: scraper::Html = scraper::Html::parse_document(&response_data);
        let selector = scraper::Selector::parse("#pagedata").unwrap();
        let selection = html.select(&selector).next().unwrap();
        let attr = selection.attr("data-blob").unwrap();

        let bandcamp_data: ParsedBandcampData = serde_json::from_str(attr).unwrap();
        if bandcamp_data.digital_items.is_empty() {
            return Ok(None);
        }

        Ok(Some(bandcamp_data.digital_items[0].clone()))
    }

    pub async fn get_digital_download_link(
        &self,
        digital_item: &DigitalItem,
        download_format: &str,
    ) -> Result<String, reqwest::Error> {
        let unqualified_link = self
            .get_unqualified_digital_download_link(digital_item, download_format)
            .unwrap_or(String::from("https://google.com"));
        self.qualify_digital_download_link(&unqualified_link).await
    }

    pub fn get_unqualified_digital_download_link(
        &self,
        digital_item: &DigitalItem,
        download_format: &str,
    ) -> Option<String> {
        digital_item.downloads.as_ref()?;

        let digital_download_list = digital_item.downloads.as_ref().unwrap();
        if digital_download_list.is_empty() || !digital_download_list.contains_key(download_format)
        {
            return None;
        }

        return Some(
            digital_download_list
                .get(download_format)
                .unwrap()
                .url
                .clone(),
        );
    }

    pub async fn qualify_digital_download_link(
        &self,
        download_link: &str,
    ) -> Result<String, reqwest::Error> {
        let stat_response_body = self
            .retrieve_digital_download_stat_data(download_link)
            .await?;
        let url = self.get_digital_download_url(&stat_response_body).unwrap();

        Ok(url)
    }

    pub async fn retrieve_digital_download_stat_data(
        &self,
        download_link: &str,
    ) -> Result<String, reqwest::Error> {
        let stat_download_url = download_link
            .replace("/download/", "/statdownload/")
            .replace("http://", "https://")
            + "&.vrs=1"
            + "&.rand="
            + &fastrand::i32(..).to_string();
        let stat_download_response = self.client.get(stat_download_url).send().await?;
        let stat_download_response_body = stat_download_response.text().await?;

        Ok(stat_download_response_body)
    }

    pub fn get_digital_download_url(
        &self,
        stat_response_body: &str,
    ) -> Result<String, regex::Error> {
        let captures = stat_response_regex().captures(stat_response_body).unwrap();
        let inner_json = captures.get(1).unwrap().as_str();
        let inner_data: ParsedStatDownload = serde_json::from_str(inner_json).unwrap();
        let download_link = inner_data.download_url.unwrap();
        Ok(download_link)
    }
}
