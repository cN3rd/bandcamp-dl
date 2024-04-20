use std::{collections::HashMap, sync::OnceLock};

use regex::Regex;

pub struct DownloadCacheRelease {
    release_id: String,
    title: String,
    year: i32,
    artist: String,
}

pub fn cache_line_regex() -> &'static Regex {
    static CACHE_LINE_REGEX: OnceLock<Regex> = OnceLock::new();
    CACHE_LINE_REGEX.get_or_init(|| {
        Regex::new(r#"(\w+)\|\s*"((?:[^"\\]*(?:\\.)?)*)" \((\w+)\) by (.*)"#).unwrap()
    })
}

pub fn read_download_cache_line(cache_line: &str) -> DownloadCacheRelease {
    let captures = cache_line_regex().captures(cache_line).unwrap();

    let release = DownloadCacheRelease {
        release_id: captures.get(1).unwrap().as_str().to_owned(),
        title: captures.get(2).unwrap().as_str().to_owned(),
        year: captures.get(3).unwrap().as_str().parse().unwrap(),
        artist: captures.get(4).unwrap().as_str().to_owned(),
    };

    release
}

pub fn read_download_cache(cache_data: &str) -> HashMap<String, DownloadCacheRelease> {
    cache_data
        .lines()
        .map(read_download_cache_line)
        .map(|c| (c.release_id.clone(), c))
        .collect::<HashMap<String, DownloadCacheRelease>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_read_download_cache_regular() {
        let cache_line: &str = r#"p199396767| "Galerie" (2022) by Anomalie"#;
        let cached_release = read_download_cache_line(cache_line);

        assert_eq!(cached_release.release_id, "p199396767");
        assert_eq!(cached_release.title, "Galerie");
        assert_eq!(cached_release.year, 2022);
        assert_eq!(cached_release.artist, "Anomalie");
    }

    #[test]
    pub fn test_read_download_cache_with_escaping() {
        let cache_line: &str = r#"p204514015| "Toxic \"Violet\" Cubes [From BSWC2021 Grand Finals]" (2021) by かめりあ(Camellia)"#;
        let cached_release = read_download_cache_line(cache_line);

        assert_eq!(cached_release.release_id, "p204514015");
        assert_eq!(
            cached_release.title,
            "Toxic \\\"Violet\\\" Cubes [From BSWC2021 Grand Finals]"
        );
        assert_eq!(cached_release.year, 2021);
        assert_eq!(cached_release.artist, "かめりあ(Camellia)");
    }

    #[test]
    pub fn test_read_download_cache_from_file() {
        let data = include_str!("data/bandcamp-collection-downloader.cache");
        let cache = read_download_cache(data);

        assert!(cache.contains_key("p225359366"));
    }
}
