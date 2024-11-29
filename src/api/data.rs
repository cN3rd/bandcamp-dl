use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::str::FromStr;

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
pub struct ParsedFanCollectionSummary {
    pub fan_id: i64,
    pub collection_summary: FanCollectionSummary,
}

#[derive(Serialize, Deserialize)]
pub struct FanCollectionSummary {
    pub fan_id: i64,
    pub username: String,
    pub url: String,
    pub tralbum_lookup: Option<HashMap<String, TrAlbumLookupItem>>,
    pub followers: Option<Vec<()>>, // TODO
}

#[derive(Serialize, Deserialize)]
pub struct TrAlbumLookupItem {
    pub item_type: String,
    pub item_id: i64,
    pub band_id: i64,
    pub purchased: String,
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
    pub last_token: Option<String>,
    pub redownload_urls: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct ParsedBandcampData {
    pub digital_items: Vec<DigitalItem>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadData {
    pub size_mb: Option<String>,
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
    pub result: Option<String>,
    pub download_url: Option<String>,
    pub url: String,
}
