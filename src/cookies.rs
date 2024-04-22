use cookie::{time::OffsetDateTime, Expiration, SameSite};
use miniserde::{Deserialize, Serialize};
use reqwest::Url;
use thiserror::Error;

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
    pub fn new(name: String, content: String) -> Self {
        Self {
            name_raw: name,
            content_raw: content,
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

        let same_site = match value.same_site_raw.as_deref() {
            Some("no_restriction") => Some(SameSite::None),
            Some("lax") => Some(SameSite::Lax),
            Some("strict") => Some(SameSite::Strict),
            Some("unspecified") => None,
            _ => None,
        };
        cookie.set_same_site(same_site);

        let expiration = value.expires_raw.and_then(|expires_str| {
            expires_str
                .parse::<i64>()
                .ok()
                .and_then(|unix_timestamp| OffsetDateTime::from_unix_timestamp(unix_timestamp).ok())
                .map(Expiration::DateTime)
                .or(Some(Expiration::Session))
        });
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
