use clap::ValueEnum;
use regex_lite::Regex;
use reqwest::Client;
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, OnceLock},
};

use crate::error::{
    ContextCreationError, DigitalDownloadError, InformationRetrievalError, ReleaseRetrievalError,
};

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Debug, ValueEnum)]
pub enum DownloadFormat {
    #[serde(rename = "mp3-v0")]
    Mp3_V0,

    #[serde(rename = "mp3-320")]
    Mp3_320,

    #[serde(rename = "flac")]
    Flac,

    #[serde(rename = "aac-hi")]
    Aac,

    #[serde(rename = "vorbis")]
    Vorbis,

    #[serde(rename = "alac")]
    Alac,

    #[serde(rename = "wav")]
    Wav,

    #[serde(rename = "aiff-lossless")]
    AiffLossless,
}

impl std::fmt::Display for DownloadFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Mp3_V0 => "mp3-v0",
            Self::Mp3_320 => "mp3-320",
            Self::Flac => "flac",
            Self::Aac => "aac-hi",
            Self::Vorbis => "vorbis",
            Self::Alac => "alac",
            Self::Wav => "wav",
            Self::AiffLossless => "aiff-lossless",
        };
        write!(f, "({str})")
    }
}

#[derive(Debug)]
pub struct ParseDownloadFormatError;

impl FromStr for DownloadFormat {
    type Err = ParseDownloadFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mp3-v0" => Ok(Self::Mp3_V0),
            "mp3-320" => Ok(Self::Mp3_320),
            "flac" => Ok(Self::Flac),
            "aac-hi" => Ok(Self::Aac),
            "vorbis" => Ok(Self::Vorbis),
            "alac" => Ok(Self::Alac),
            "wav" => Ok(Self::Wav),
            "aiff-lossless" => Ok(Self::AiffLossless),
            _ => Err(ParseDownloadFormatError),
        }
    }
}

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
    pub item_count: Option<i64>,
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
    pub downloads: Option<HashMap<DownloadFormat, DownloadData>>,
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

pub type SaleIdUrlMap = HashMap<String, String>;

impl BandcampAPIContext {
    pub fn new(user_name: &str, cookie_data: &str) -> Result<Self, ContextCreationError> {
        let cookie_store = crate::cookies::read_json_file(cookie_data, "https://bandcamp.com")?;
        let client = Client::builder()
            .cookie_provider(Arc::new(CookieStoreMutex::new(cookie_store)))
            .build()?;
        let user_name = user_name.to_owned();

        Ok(Self { client, user_name })
    }

    pub async fn get_fanpage_data(&self) -> Result<ParsedFanpageData, InformationRetrievalError> {
        let response = self
            .client
            .get(format!("https://bandcamp.com/{}", self.user_name))
            .send()
            .await?;
        let response_body = response.text().await?;

        let data_blob = data_blob_regex()
            .captures(&response_body)
            .ok_or(InformationRetrievalError::DataBlobNotFound)?
            .get(1)
            .ok_or(InformationRetrievalError::DataBlobNotFound)?
            .as_str();
        let data_blob = htmlize::unescape(data_blob);

        Ok(serde_json::from_str::<ParsedFanpageData>(&data_blob)?)
    }

    pub async fn get_all_releases(
        &self,
        fanpage_data: &ParsedFanpageData,
        include_hidden: bool,
    ) -> Result<SaleIdUrlMap, ReleaseRetrievalError> {
        let mut all_downloads = fanpage_data
            .collection_data
            .redownload_urls
            .clone()
            .ok_or(ReleaseRetrievalError::NoDownloadLinksFound)?;

        if all_downloads.is_empty() {
            return Err(ReleaseRetrievalError::NoDownloadLinksFound);
        }

        if !include_hidden {
            let hidden_items = &fanpage_data.item_cache.hidden;
            all_downloads = all_downloads
                .into_iter()
                .filter(|(k, _)| !hidden_items.contains_key(k))
                .collect::<HashMap<_, _>>(); // TODO: fix this
        }

        // Get the rest of the non-hidden collection
        if let Some(item_count) = fanpage_data.collection_data.item_count {
            if item_count > fanpage_data.collection_data.batch_size {
                let fan_id: i64 = fanpage_data.fan_data.fan_id;
                if let Some(last_token) = &fanpage_data.collection_data.last_token {
                    all_downloads.extend(
                        self.get_webui_download_urls(fan_id, last_token, "collection_items")
                            .await?
                            .into_iter(),
                    );
                }

                if include_hidden {
                    if let Some(last_token) = &fanpage_data.hidden_data.last_token {
                        all_downloads.extend(
                            self.get_webui_download_urls(fan_id, last_token, "hidden_items")
                                .await?
                                .into_iter(),
                        );
                    }
                }
            }
        }

        Ok(all_downloads)
    }

    pub async fn get_webui_download_urls(
        &self,
        fan_id: i64,
        last_token: &str,
        collection_name: &str,
    ) -> Result<SaleIdUrlMap, ReleaseRetrievalError> {
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
                serde_json::from_str(&response_data)?;

            download_urls.extend(parsed_collection_data.redownload_urls.into_iter());

            more_available = parsed_collection_data.more_available;
            last_token = parsed_collection_data.last_token;
        }

        Ok(download_urls)
    }

    pub async fn get_digital_download_item(
        &self,
        item_url: &str,
    ) -> Result<Option<DigitalItem>, InformationRetrievalError> {
        let response = self.client.get(item_url).send().await?;
        let response_data = response.text().await?;

        let data_blob = data_blob_regex()
            .captures(&response_data)
            .ok_or(InformationRetrievalError::DataBlobNotFound)?
            .get(1)
            .ok_or(InformationRetrievalError::DataBlobNotFound)?
            .as_str();
        let data_blob = htmlize::unescape(data_blob);

        let bandcamp_data = serde_json::from_str::<ParsedBandcampData>(&data_blob)?;
        if bandcamp_data.digital_items.is_empty() {
            return Ok(None);
        }

        Ok(Some(bandcamp_data.digital_items[0].clone()))
    }

    pub async fn get_digital_download_link(
        &self,
        digital_item: &DigitalItem,
        download_format: DownloadFormat,
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
        get_qualified_digital_download_url(
            &self
                .retrieve_digital_download_stat_data(download_link)
                .await?,
        )
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
        let stat_download_response = self.client.get(stat_download_url).send().await?;
        let stat_download_response_body = stat_download_response.text().await?;

        Ok(stat_download_response_body)
    }
}

pub fn get_unqualified_digital_download_link(
    digital_item: &DigitalItem,
    download_format: DownloadFormat,
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

    let inner_data: ParsedStatDownload = serde_json::from_str(inner_json)?;

    inner_data
        .download_url
        .ok_or(DigitalDownloadError::NoLinkFound)
}
