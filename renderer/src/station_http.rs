use std::time::Duration;

use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue, ETAG, IF_NONE_MATCH};
use reqwest::{StatusCode, Url};

use crate::station_source::sanitize_url;

const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, thiserror::Error)]
pub enum HttpSourceError {
    #[error("HTTP clientを構築できない: {0}")]
    Client(#[source] reqwest::Error),

    #[error("HTTPリクエストに失敗: {0}")]
    Request(#[source] reqwest::Error),

    #[error("想定外のHTTP status: {0}")]
    UnexpectedStatus(StatusCode),

    #[error(transparent)]
    Load(#[from] renderer_assets::StationLoadError),

    #[error("blocking taskの実行に失敗: {0}")]
    Join(#[from] tokio::task::JoinError),
}

enum Fetched {
    NotModified,
    Body {
        body: Bytes,
        etag: Option<HeaderValue>,
    },
}

pub struct HttpSource {
    client: reqwest::Client,
    url: Url,
    headers: HeaderMap,
    interval: Duration,
    etag: Option<HeaderValue>,
}

pub fn prepare(
    url: Url,
    headers: HeaderMap,
    interval: Duration,
) -> Result<HttpSource, HttpSourceError> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .connect_timeout(TIMEOUT)
        .user_agent(concat!("eew-renderer/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(HttpSourceError::Client)?;

    Ok(HttpSource {
        client,
        url,
        headers,
        interval,
        etag: None,
    })
}

impl HttpSource {
    async fn fetch(&self, etag: Option<&HeaderValue>) -> Result<Fetched, HttpSourceError> {
        let mut request = self
            .client
            .get(self.url.clone())
            .headers(self.headers.clone());

        if let Some(etag) = etag {
            request = request.header(IF_NONE_MATCH, etag);
        }

        let response = request
            .send()
            .await
            .map_err(|e| HttpSourceError::Request(e.without_url()))?;

        let status = response.status();

        if status == StatusCode::NOT_MODIFIED {
            return Ok(Fetched::NotModified);
        }

        if status != StatusCode::OK {
            return Err(HttpSourceError::UnexpectedStatus(status));
        }

        let etag = response.headers().get(ETAG).cloned();

        let body = response
            .bytes()
            .await
            .map_err(|e| HttpSourceError::Request(e.without_url()))?;

        Ok(Fetched::Body { body, etag })
    }

    pub async fn initial_load(&mut self) -> Result<(), HttpSourceError> {
        let (body, etag) = match self.fetch(None).await? {
            Fetched::NotModified => {
                return Err(HttpSourceError::UnexpectedStatus(StatusCode::NOT_MODIFIED))
            }
            Fetched::Body { body, etag } => (body, etag),
        };

        tokio::task::spawn_blocking(move || {
            renderer_assets::QueryInterface::init_intensity_stations(&body)
        })
        .await??;

        self.etag = etag;

        Ok(())
    }

    pub fn start(mut self) -> tokio::task::JoinHandle<()> {
        let url = sanitize_url(&self.url);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.interval).await;

                match self.fetch(self.etag.as_ref()).await {
                    Ok(Fetched::NotModified) => {
                        tracing::debug!("Intensity stations HTTP source not modified: {url}");
                    }

                    Ok(Fetched::Body { body, etag }) => {
                        let result = tokio::task::spawn_blocking(move || {
                            renderer_assets::QueryInterface::replace_intensity_stations(&body)
                        })
                        .await;

                        match result {
                            Ok(Ok(())) => {
                                self.etag = etag;
                                tracing::info!(
                                    "Reloaded intensity stations from HTTP source: {url}"
                                );
                            }
                            Ok(Err(e)) => tracing::error!(
                                "Failed to validate intensity stations from HTTP source; keeping previous data: {e:?}"
                            ),
                            Err(e) => tracing::error!(
                                "Failed to validate intensity stations from HTTP source; keeping previous data: {e:?}"
                            ),
                        }
                    }

                    Err(e) => tracing::error!(
                        "Failed to fetch intensity stations from HTTP source; keeping previous data: {e}"
                    ),
                }
            }
        })
    }
}
