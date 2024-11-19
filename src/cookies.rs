use crate::error::CookieJsonParsingError;
use cookie::{time::OffsetDateTime, Expiration, SameSite};
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RawCookie {
    pub name: String,
    pub value: String,

    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub expires: Option<String>,

    #[serde(default)]
    pub send_for: Option<String>,
    #[serde(default)]
    pub http_only: Option<String>,
    #[serde(default)]
    pub same_site: Option<String>,
    #[serde(default)]
    pub this_domain_only: Option<String>,

    pub store: Option<String>,
}

impl RawCookie {
    pub const fn new(name: String, value: String) -> Self {
        Self {
            name,
            value,
            host: None,
            path: None,
            expires: None,
            send_for: None,
            http_only: None,
            same_site: None,
            this_domain_only: None,
            store: None,
        }
    }
}

fn parse_expiration(expire_str_option: Option<&str>) -> Option<Expiration> {
    expire_str_option.and_then(|expires| {
        expires
            .parse()
            .ok()
            .and_then(|ts| OffsetDateTime::from_unix_timestamp(ts).ok())
            .map(Expiration::DateTime)
    })
}

fn parse_same_site(same_site: Option<&str>) -> Option<SameSite> {
    match same_site {
        Some("no_restriction") => Some(SameSite::None),
        Some("lax") => Some(SameSite::Lax),
        Some("strict") => Some(SameSite::Strict),
        // Some("unspecified") => None,
        _ => None,
    }
}

fn parse_bool_option(value: Option<&String>) -> Option<bool> {
    value.and_then(|v| v.parse().ok())
}

impl From<RawCookie> for cookie::Cookie<'_> {
    fn from(raw_cookie: RawCookie) -> Self {
        let mut cookie = cookie::Cookie::new(raw_cookie.name, raw_cookie.value);

        if let Some(host_raw) = raw_cookie.host {
            cookie.set_domain(
                host_raw
                    .replace("https://.", "")
                    .replace("http://.", "")
                    .replace('/', ""),
            );
        }
        if let Some(path_raw) = raw_cookie.path {
            cookie.set_path(path_raw);
        }

        if let Some(secure) = parse_bool_option(raw_cookie.send_for.as_ref()) {
            cookie.set_secure(secure);
        }

        if let Some(http_only) = parse_bool_option(raw_cookie.http_only.as_ref()) {
            cookie.set_http_only(http_only);
        }

        let same_site = parse_same_site(raw_cookie.same_site.as_deref());
        cookie.set_same_site(same_site);

        let expiration = parse_expiration(raw_cookie.expires.as_deref());
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

    let cookies: Vec<RawCookie> = serde_json::from_str(cookie_data)?;

    Ok(cookie_store::CookieStore::from_cookies(
        cookies
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
        let cookie_data = RawCookie::new("name".into(), "value".into());

        assert_eq!(cookie_data.name, "name");
        assert_eq!(cookie_data.value, "value");
    }

    #[test]
    pub fn cookie_from_minimal_ourcookie_ok() {
        let cookie_data = RawCookie::new("name".into(), "value".into());
        let cookie = Cookie::from(cookie_data);

        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "value");
    }

    #[test]
    pub fn cookie_from_complex_ourcookie_ok() {
        let cookie_data = RawCookie {
            name: "fan_visits".to_owned(),
            value: "1234567".to_owned(),
            host: Some("http://.bandcamp.com/".to_owned()),
            path: Some("/".to_owned()),
            expires: Some("1919434332".to_owned()),
            send_for: Some("false".to_owned()),
            http_only: Some("false".to_owned()),
            same_site: Some("no_restriction".to_owned()),
            this_domain_only: Some("false".to_owned()),
            store: Some("firefox-default".to_owned()),
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
        let cookie_data = RawCookie {
            name: "fan_visits".to_owned(),
            value: "1234567".to_owned(),
            host: Some("http://.bandcamp.com/".to_owned()),
            path: None,
            expires: None,
            send_for: Some("false".to_owned()),
            http_only: Some("false".to_owned()),
            same_site: None,
            this_domain_only: Some("false".to_owned()),
            store: Some("firefox-default".to_owned()),
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
