use std::{collections::HashMap, path::Path, sync::OnceLock};

use regex::Regex;

use crate::cache;

pub struct DownloadCacheRelease {
    release_id: String,
    title: String,
    year: i32,
    artist: String,
}

pub fn read_download_cache_line(cache_line: &str) -> DownloadCacheRelease {
    static CACHE_LINE_REGEX_STATIC: OnceLock<Regex> = OnceLock::new();
    let cache_line_regex = CACHE_LINE_REGEX_STATIC.get_or_init(|| {
        Regex::new(r#"(\w+)\|\s*"((?:[^"\\]*(?:\\.)?)*)" \((\w+)\) by (.*)"#).unwrap()
    });

    let captures = cache_line_regex.captures(cache_line).unwrap();

    let release = DownloadCacheRelease {
        release_id: captures.get(1).unwrap().as_str().to_owned(),
        title: captures.get(2).unwrap().as_str().to_owned(),
        year: captures.get(3).unwrap().as_str().parse().unwrap(),
        artist: captures.get(4).unwrap().as_str().to_owned(),
    };

    release
}

type DownloadCache = HashMap<String, DownloadCacheRelease>;

pub fn read_download_cache(cache_data: &str) -> DownloadCache {
    cache_data
        .lines()
        .map(read_download_cache_line)
        .map(|c| (c.release_id.clone(), c))
        .collect()
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

    #[test]
    pub fn test_read_download_cache_regular() {
        let cache_line = r#"p199396767| "Galerie" (2022) by Anomalie"#;
        let cache_release = read_download_cache_line(cache_line);

        assert_eq!(cache_release.release_id, "p199396767");
        assert_eq!(cache_release.title, "Galerie");
        assert_eq!(cache_release.year, 2022);
        assert_eq!(cache_release.artist, "Anomalie");
    }

    #[test]
    pub fn test_read_download_cache_with_escaping() {
        let cache_line = r#"p204514015| "Toxic \"Violet\" Cubes [From BSWC2021 Grand Finals]" (2021) by かめりあ(Camellia)"#;
        let cache_release = read_download_cache_line(cache_line);

        assert_eq!(cache_release.release_id, "p204514015");
        assert_eq!(
            cache_release.title,
            "Toxic \\\"Violet\\\" Cubes [From BSWC2021 Grand Finals]"
        );
        assert_eq!(cache_release.year, 2021);
        assert_eq!(cache_release.artist, "かめりあ(Camellia)");
    }

    // TODO: bad cases for read_download_cache_line, serialize_download_cache_release

    #[test]
    pub fn test_read_download_cache_from_file() {
        let data = include_str!("data/bandcamp-collection-downloader.cache");
        let cache = read_download_cache(data);

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

        assert_eq!(deserialized_release.release_id, cache_release.release_id);
        assert_eq!(deserialized_release.title, cache_release.title);
        assert_eq!(deserialized_release.year, cache_release.year);
        assert_eq!(deserialized_release.artist, cache_release.artist);
    }
}
