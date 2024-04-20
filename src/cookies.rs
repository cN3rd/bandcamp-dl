use cookie::{time::OffsetDateTime, Expiration, SameSite};
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
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
    pub this_domain_only_raw: String,

    #[serde(rename = "Store raw")]
    pub store_raw: Option<String>,
}

impl From<OurCookie> for cookie::Cookie<'_> {
    fn from(value: OurCookie) -> Self {
        let same_site = match value.same_site_raw.unwrap().as_str() {
            "no_restriction" => Some(SameSite::None),
            "lax" => Some(SameSite::Lax),
            "strict" => Some(SameSite::Strict),
            "unspecified" => None,
            _ => None,
        };

        let expiration = if let Ok(unix_timestamp) = value.expires_raw.unwrap().parse() {
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
                .unwrap()
                .replace("https://.", "")
                .replace("http://.", "")
                .replace('/', ""),
        );
        cookie.set_path(value.path_raw.unwrap());
        cookie.set_secure(value.send_for_raw.unwrap().parse().ok());
        cookie.set_http_only(value.http_only_raw.unwrap().parse().ok());
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
