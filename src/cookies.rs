use cookie::{time::OffsetDateTime, Expiration, SameSite};
use miniserde::{Deserialize, Serialize};
use reqwest::Url;

use crate::error::CookieJsonParsingError;

#[derive(Debug, Serialize, Deserialize)]
pub struct OurCookie {
    #[serde(rename = "Name raw")]
    pub name_raw: String,
    #[serde(rename = "Content raw")]
    pub content_raw: String,

    #[serde(rename = "Host raw")]
    pub host_raw: Option<String>,
    #[serde(rename = "Path raw")]
    pub path_raw: Option<String>,

    #[serde(rename = "Expires raw")]
    pub expires_raw: Option<String>,

    #[serde(rename = "Send for raw")]
    pub send_for_raw: Option<String>,
    #[serde(rename = "HTTP only raw")]
    pub http_only_raw: Option<String>,
    #[serde(rename = "SameSite raw")]
    pub same_site_raw: Option<String>,
    #[serde(rename = "This domain only raw")]
    pub this_domain_only_raw: Option<String>,

    #[serde(rename = "Store raw")]
    pub store_raw: Option<String>,
}

impl OurCookie {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name_raw: name.to_owned(),
            content_raw: content.to_owned(),
            host_raw: None,
            path_raw: None,
            expires_raw: None,
            send_for_raw: None,
            http_only_raw: None,
            same_site_raw: None,
            this_domain_only_raw: None,
            store_raw: None,
        }
    }
}

fn parse_expiration(expire_str_option: Option<&str>) -> Option<Expiration> {
    expire_str_option.and_then(|expires_str| {
        expires_str
            .parse::<i64>()
            .ok()
            .and_then(|unix_timestamp| OffsetDateTime::from_unix_timestamp(unix_timestamp).ok())
            .map(Expiration::DateTime)
    })
}

fn parse_same_site(same_site: Option<&str>) -> Option<SameSite> {
    match same_site {
        Some("no_restriction") => Some(SameSite::None),
        Some("lax") => Some(SameSite::Lax),
        Some("strict") => Some(SameSite::Strict),
        Some("unspecified") => None,
        _ => None,
    }
}

impl From<OurCookie> for cookie::Cookie<'_> {
    fn from(value: OurCookie) -> Self {
        let mut cookie = cookie::Cookie::new(value.name_raw, value.content_raw);

        if let Some(host_raw) = value.host_raw {
            cookie.set_domain(
                host_raw
                    .replace("https://.", "")
                    .replace("http://.", "")
                    .replace('/', ""),
            );
        }
        if let Some(path_raw) = value.path_raw {
            cookie.set_path(path_raw);
        }
        if let Some(send_for_raw) = value.send_for_raw {
            cookie.set_secure(send_for_raw.parse().ok());
        }
        if let Some(http_only_raw) = value.http_only_raw {
            cookie.set_http_only(http_only_raw.parse().ok());
        }

        let same_site = parse_same_site(value.same_site_raw.as_deref());
        cookie.set_same_site(same_site);

        let expiration = parse_expiration(value.expires_raw.as_deref());
        if let Some(expiration) = expiration {
            cookie.set_expires(expiration);
        }

        cookie
    }
}

pub fn read_json_file(
    cookie_data: &str,
    request_url: &str,
) -> Result<cookie_store::CookieStore, CookieJsonParsingError> {
    let request_url = Url::parse(request_url)
        .map_err(|err| CookieJsonParsingError::InvalidUrlProvided(err.to_string()))?;

    Ok(cookie_store::CookieStore::from_cookies(
        miniserde::json::from_str::<Vec<OurCookie>>(cookie_data)?
            .into_iter()
            .map(cookie::Cookie::from)
            .map(|c| cookie_store::Cookie::try_from_raw_cookie(&c, &request_url)),
        false,
    )?)
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_matches::assert_matches;
    use cookie::Cookie;
    use test_case::test_case;

    #[test_case("google.com")]
    #[test_case("bandcamp.com")]
    #[test_case("notreallybandcamp.com")]
    pub fn test_read_json_file_url_error(url: &str) {
        let result = read_json_file("", url);

        assert!(result.is_err());
        let err: CookieJsonParsingError = result.unwrap_err();

        assert_matches!(err, CookieJsonParsingError::InvalidUrlProvided(_));
    }

    #[test_case("Not a json file :(")]
    #[test_case("[{\"test\": \"not a proper cookie\"}]")]
    #[test_case("{\"hello\": \"world\"}")]
    pub fn test_read_json_invalid(invalid_cookie_data: &str) {
        let result = read_json_file(invalid_cookie_data, "https://bandcamp.com");

        assert!(result.is_err());
        let err: CookieJsonParsingError = result.unwrap_err();

        assert_matches!(err, CookieJsonParsingError::JsonParsingError(_));
    }

    #[test]
    fn test_parse_same_site() {
        assert_eq!(
            parse_same_site(Some("no_restriction")),
            Some(SameSite::None),
        );
        assert_eq!(parse_same_site(Some("lax")), Some(SameSite::Lax));
        assert_eq!(parse_same_site(Some("strict")), Some(SameSite::Strict));
        assert_eq!(parse_same_site(Some("unspecified")), None);

        assert_eq!(parse_same_site(Some("truly_unknown_mode")), None);
        assert_eq!(parse_same_site(Some("happiness")), None);
        assert_eq!(parse_same_site(None), None);
    }

    #[test]
    fn test_parse_expiration() {
        assert_eq!(parse_expiration(None), None);
        assert_eq!(parse_expiration(Some("")), None);
        assert_eq!(parse_expiration(Some("invalid")), None);

        let timestamp = 1609459200; // Represents 2021-01-01 00:00:00 UTC
        let expected_date = OffsetDateTime::from_unix_timestamp(timestamp).unwrap();
        assert_eq!(
            parse_expiration(Some("1609459200")),
            Some(Expiration::DateTime(expected_date))
        );
    }

    #[test]
    pub fn ourcookie_new_ok() {
        let cookie_data = OurCookie::new("name", "content");

        assert_eq!(cookie_data.name_raw, "name");
        assert_eq!(cookie_data.content_raw, "content");
    }

    #[test]
    pub fn cookie_from_minimal_ourcookie_ok() {
        let cookie_data = OurCookie::new("name", "content");
        let cookie = Cookie::from(cookie_data);

        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "content");
    }

    #[test]
    pub fn cookie_from_complex_ourcookie_ok() {
        let cookie_data = OurCookie {
            name_raw: "fan_visits".to_owned(),
            content_raw: "1234567".to_owned(),
            host_raw: Some("http://.bandcamp.com/".to_owned()),
            path_raw: Some("/".to_owned()),
            expires_raw: Some("1919434332".to_owned()),
            send_for_raw: Some("false".to_owned()),
            http_only_raw: Some("false".to_owned()),
            same_site_raw: Some("no_restriction".to_owned()),
            this_domain_only_raw: Some("false".to_owned()),
            store_raw: Some("firefox-default".to_owned()),
        };
        let cookie = Cookie::from(cookie_data);

        assert_eq!(cookie.name(), "fan_visits");
        assert_eq!(cookie.value(), "1234567");

        assert_eq!(cookie.expires(), parse_expiration(Some("1919434332")));

        assert_eq!(cookie.domain(), Some("bandcamp.com"));
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.secure(), Some(false));
        assert_eq!(cookie.http_only(), Some(false));
    }

    #[test]
    pub fn cookie_from_partial_ourcookie_ok() {
        let cookie_data = OurCookie {
            name_raw: "fan_visits".to_owned(),
            content_raw: "1234567".to_owned(),
            host_raw: Some("http://.bandcamp.com/".to_owned()),
            path_raw: None,
            expires_raw: None,
            send_for_raw: Some("false".to_owned()),
            http_only_raw: Some("false".to_owned()),
            same_site_raw: None,
            this_domain_only_raw: Some("false".to_owned()),
            store_raw: Some("firefox-default".to_owned()),
        };
        let cookie = Cookie::from(cookie_data);

        assert_eq!(cookie.name(), "fan_visits");
        assert_eq!(cookie.value(), "1234567");

        assert_eq!(cookie.expires(), None);

        assert_eq!(cookie.domain(), Some("bandcamp.com"));
        assert_eq!(cookie.path(), None);
        assert_eq!(cookie.secure(), Some(false));
        assert_eq!(cookie.http_only(), Some(false));
    }
}
