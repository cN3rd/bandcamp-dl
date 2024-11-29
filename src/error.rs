use thiserror::Error;

#[derive(Debug, Error)]
pub enum CookieJsonParsingError {
    #[error("Invalid store url provided: {0}")]
    InvalidUrlProvided(String),

    #[error("Cookie parsing error: {0}")]
    CookieParsingError(#[from] cookie_store::CookieError),

    #[error("Json parsing error: {0}")]
    JsonParsingError(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ContextCreationError {
    #[error("Cookie file parsing error: {0}")]
    CookieParsingError(#[from] CookieJsonParsingError),

    #[error("HTTP client creation error: {0}")]
    ClientCreationError(#[from] reqwest::Error),
}

#[derive(Debug, Error)]
pub enum InformationRetrievalError {
    #[error("HTTP requesting error: {0}")]
    HttpRequestError(#[from] reqwest::Error),

    #[error("HTTP requesting error: {0}")]
    HttpMiddlewareRequestError(#[from] reqwest_middleware::Error),

    #[error("Json parsing error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Data blob not found")]
    DataBlobNotFound,
}

#[derive(Debug, Error)]
pub enum ReleaseRetrievalError {
    #[error("HTTP requesting error: {0}")]
    HttpRequestError(#[from] reqwest::Error),

    #[error("HTTP requesting error: {0}")]
    HttpMiddlewareRequestError(#[from] reqwest_middleware::Error),

    #[error("Json parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("No download links found")]
    NoDownloadLinksFound,
}

#[derive(Error, Debug)]
pub enum DigitalDownloadError {
    #[error("Failed to pull links due to JSON error, with retry url: {0}")]
    JsonResponseErrorCode(String),

    #[error("HTTP requesting error: {0}")]
    HttpRequestError(#[from] reqwest::Error),

    #[error("HTTP requesting error: {0}")]
    HttpMiddlewareRequestError(#[from] reqwest_middleware::Error),

    #[error("Json parsing error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Failed to find json body")]
    JsonBodyNotFound,

    #[error("No download links found")]
    NoDownloadLinksFound,

    #[error("No qualified download link found")]
    NoLinkFound,

    #[error("Download link in requested format not found")]
    RequestedFormatLinkNotFound,
}
