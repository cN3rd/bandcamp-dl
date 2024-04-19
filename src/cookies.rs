use cookie::{time::OffsetDateTime, Expiration, SameSite};
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct OurCookie {
    #[serde(rename = "Name raw")]
    pub name_raw: String,
    #[serde(rename = "Content raw")]
    pub content_raw: String,

    #[serde(rename = "Host raw")]
    pub host_raw: String,
    #[serde(rename = "Path raw")]
    pub path_raw: String,

    #[serde(rename = "Expires")]
    pub expires: String,
    #[serde(rename = "Expires raw")]
    pub expires_raw: String,

    #[serde(rename = "Send for")]
    pub send_for: String,
    #[serde(rename = "Send for raw")]
    pub send_for_raw: String,
    #[serde(rename = "HTTP only raw")]
    pub http_only_raw: String,
    #[serde(rename = "SameSite raw")]
    pub same_site_raw: String,
    #[serde(rename = "This domain only")]
    pub this_domain_only: String,
    #[serde(rename = "This domain only raw")]
    pub this_domain_only_raw: String,

    #[serde(rename = "Store raw")]
    pub store_raw: Option<String>,
    #[serde(rename = "First Party Domain")]
    pub first_party_domain: Option<String>,
}

impl From<OurCookie> for cookie::Cookie<'_> {
    fn from(value: OurCookie) -> Self {
        let same_site = match value.same_site_raw.as_str() {
            "no_restriction" => Some(SameSite::None),
            "lax" => Some(SameSite::Lax),
            "strict" => Some(SameSite::Strict),
            "unspecified" => None,
            _ => None,
        };

        let expiration = if let Ok(unix_timestamp) = value.expires_raw.parse() {
            if let Ok(timestamp) = OffsetDateTime::from_unix_timestamp(unix_timestamp) {
                Expiration::DateTime(timestamp)
            } else {
                Expiration::Session
            }
        } else {
            Expiration::Session
        };

        let mut cookie = cookie::Cookie::new(value.name_raw, value.content_raw);
        cookie.set_domain(
            value
                .host_raw
                .replace("https://.", "")
                .replace("http://.", "")
                .replace('/', ""),
        );
        cookie.set_path(value.path_raw);
        cookie.set_secure(value.send_for_raw.parse().ok());
        cookie.set_http_only(value.http_only_raw.parse().ok());
        cookie.set_same_site(same_site);

        cookie.set_expires(expiration);

        cookie
    }
}

pub fn read_json_file(cookie_data: &str, request_url: &str) -> cookie_store::CookieStore {
    let request_url = Url::parse(request_url).expect("valid URL expected");

    let store_result = cookie_store::CookieStore::from_cookies(
        serde_json::from_str::<Vec<OurCookie>>(cookie_data)
            .expect("proper error handling missing")
            .into_iter()
            .map(cookie::Cookie::from)
            .map(|c| cookie_store::Cookie::try_from_raw_cookie(&c, &request_url).to_owned()),
        false,
    );

    store_result.unwrap()
}
