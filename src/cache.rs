use std::{collections::HashMap, num::ParseIntError, sync::OnceLock};

use regex_lite::Regex;
use thiserror::Error;

#[derive(Debug)]
pub struct DownloadCacheRelease {
    release_id: String,
    title: String,
    year: i32,
    artist: String,
}

#[derive(Debug, Error)]
pub enum CacheParsingError<'a> {
    #[error("Failed to match line \"{0}\" on expression")]
    RegexCaptureFail(&'a str),

    #[error("Failed to get regex group {0}")]
    RegexGroupFail(i32),

    #[error("Parse int error: {0}")]
    ParseIntError(#[from] ParseIntError),
}

pub fn read_download_cache_line(
    cache_line: &str,
) -> Result<DownloadCacheRelease, CacheParsingError> {
    static CACHE_LINE_REGEX_STATIC: OnceLock<Regex> = OnceLock::new();
    let cache_line_regex = CACHE_LINE_REGEX_STATIC.get_or_init(|| {
        Regex::new(r#"(\w+)\|\s*"((?:[^"\\]*(?:\\.)?)*)" \((\w+)\) by (.*)"#)
            .expect("CACHE_LINE_REGEX must successfully compile")
    });

    let captures = cache_line_regex
        .captures(cache_line)
        .ok_or(CacheParsingError::RegexCaptureFail(cache_line))?;

    let release = DownloadCacheRelease {
        release_id: captures
            .get(1)
            .ok_or(CacheParsingError::RegexGroupFail(1))?
            .as_str()
            .to_owned(),
        title: captures
            .get(2)
            .ok_or(CacheParsingError::RegexGroupFail(2))?
            .as_str()
            .to_owned(),
        year: captures
            .get(3)
            .ok_or(CacheParsingError::RegexGroupFail(3))?
            .as_str()
            .parse()?,
        artist: captures
            .get(4)
            .ok_or(CacheParsingError::RegexGroupFail(4))?
            .as_str()
            .to_owned(),
    };

    Ok(release)
}

type DownloadCache = HashMap<String, DownloadCacheRelease>;

pub fn read_download_cache(cache_data: &str) -> Result<DownloadCache, CacheParsingError> {
    let lines: Result<Vec<_>, _> = cache_data.lines().map(read_download_cache_line).collect();

    Ok(lines?
        .into_iter()
        .map(|c| (c.release_id.clone(), c))
        .collect())
}

pub fn serialize_download_cache_release(cache_release: &DownloadCacheRelease) -> String {
    format!(
        "{}| \"{}\" ({}) by {}",
        cache_release.release_id, cache_release.title, cache_release.year, cache_release.artist
    )
}

pub fn serialize_download_cache(cache_data: DownloadCache) -> String {
    cache_data
        .values()
        .map(serialize_download_cache_release)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    pub fn test_read_download_cache_regular() {
        let cache_line = r#"p199396767| "Galerie" (2022) by Anomalie"#;
        let cache_release = read_download_cache_line(cache_line);

        assert!(cache_release.is_ok());
        let cache_release = cache_release.unwrap();

        assert_eq!(cache_release.release_id, "p199396767");
        assert_eq!(cache_release.title, "Galerie");
        assert_eq!(cache_release.year, 2022);
        assert_eq!(cache_release.artist, "Anomalie");
    }

    #[test]
    pub fn test_read_download_cache_invalid_cases() {
        assert_matches!(
            read_download_cache("Hi this is a test"),
            Err(CacheParsingError::RegexCaptureFail(_))
        );
        assert_matches!(
            read_download_cache(r#"pewpew1234| "ABCD" (1234) by"#),
            Err(CacheParsingError::RegexCaptureFail(_))
        );
        assert_matches!(
            read_download_cache(r#"pewpew1234| "ABCD" (hello)"#),
            Err(CacheParsingError::RegexCaptureFail(_))
        );
    }

    #[test]
    pub fn test_read_download_cache_with_escaping() {
        let cache_line = r#"p204514015| "Toxic \"Violet\" Cubes [From BSWC2021 Grand Finals]" (2021) by かめりあ(Camellia)"#;
        let cache_release = read_download_cache_line(cache_line);

        assert!(cache_release.is_ok());
        let cache_release = cache_release.unwrap();

        assert_eq!(cache_release.release_id, "p204514015");
        assert_eq!(
            cache_release.title,
            "Toxic \\\"Violet\\\" Cubes [From BSWC2021 Grand Finals]"
        );
        assert_eq!(cache_release.year, 2021);
        assert_eq!(cache_release.artist, "かめりあ(Camellia)");
    }

    #[test]
    pub fn test_read_download_cache_from_file() {
        let data = include_str!("data/bandcamp-collection-downloader.cache");
        let cache = read_download_cache(data);

        assert!(cache.is_ok());
        let cache = cache.unwrap();

        assert!(cache.contains_key("p225359366"));
    }

    #[test]
    pub fn test_serialize_normal_release() {
        let cache_release = DownloadCacheRelease {
            release_id: "p199396767".to_owned(),
            title: "Galerie".to_owned(),
            year: 2022,
            artist: "Anomalie".to_owned(),
        };
        let cache_line = r#"p199396767| "Galerie" (2022) by Anomalie"#;

        assert_eq!(serialize_download_cache_release(&cache_release), cache_line);
    }

    #[test]
    pub fn test_serialize_cache_line_with_escaping() {
        let cache_release = DownloadCacheRelease {
            release_id: "p204514015".to_owned(),
            title: "Toxic \\\"Violet\\\" Cubes [From BSWC2021 Grand Finals]".to_owned(),
            year: 2021,
            artist: "かめりあ(Camellia)".to_owned(),
        };
        let cache_line = r#"p204514015| "Toxic \"Violet\" Cubes [From BSWC2021 Grand Finals]" (2021) by かめりあ(Camellia)"#;

        assert_eq!(serialize_download_cache_release(&cache_release), cache_line);
    }

    #[test]
    pub fn test_round_trip_regular() {
        let cache_release = DownloadCacheRelease {
            release_id: "p199396767".to_owned(),
            title: "Galerie".to_owned(),
            year: 2022,
            artist: "Anomalie".to_owned(),
        };

        let cache_line = serialize_download_cache_release(&cache_release);
        let deserialized_release = read_download_cache_line(&cache_line);

        assert!(deserialized_release.is_ok());
        let deserialized_release = deserialized_release.unwrap();

        assert_eq!(deserialized_release.release_id, cache_release.release_id);
        assert_eq!(deserialized_release.title, cache_release.title);
        assert_eq!(deserialized_release.year, cache_release.year);
        assert_eq!(deserialized_release.artist, cache_release.artist);
    }

    #[test]
    pub fn test_round_trip_minimal() {
        let cache_release = DownloadCacheRelease {
            release_id: "p0".to_owned(),
            title: "".to_owned(),
            year: 0,
            artist: "".to_owned(),
        };

        let cache_line = serialize_download_cache_release(&cache_release);
        let deserialized_release = read_download_cache_line(&cache_line);

        assert!(deserialized_release.is_ok());
        let deserialized_release = deserialized_release.unwrap();

        assert_eq!(deserialized_release.release_id, cache_release.release_id);
        assert_eq!(deserialized_release.title, cache_release.title);
        assert_eq!(deserialized_release.year, cache_release.year);
        assert_eq!(deserialized_release.artist, cache_release.artist);
    }

    #[test]
    pub fn test_round_trip_with_escaping() {
        let cache_release = DownloadCacheRelease {
            release_id: "p204514015".to_owned(),
            title: "Toxic \\\"Violet\\\" Cubes [From BSWC2021 Grand Finals]".to_owned(),
            year: 2021,
            artist: "かめりあ(Camellia)".to_owned(),
        };

        let cache_line = serialize_download_cache_release(&cache_release);
        let deserialized_release = read_download_cache_line(&cache_line);

        assert!(deserialized_release.is_ok());
        let deserialized_release = deserialized_release.unwrap();

        assert_eq!(deserialized_release.release_id, cache_release.release_id);
        assert_eq!(deserialized_release.title, cache_release.title);
        assert_eq!(deserialized_release.year, cache_release.year);
        assert_eq!(deserialized_release.artist, cache_release.artist);
    }
}
