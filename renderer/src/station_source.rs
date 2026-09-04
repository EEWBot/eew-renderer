use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum SourceParseError {
    #[error("空のパスは指定できない (例: file:/data/intensity_stations.json)")]
    EmptyPath,

    #[error("URLとして解釈できない: {0}")]
    Url(#[from] url::ParseError),

    #[error("sourceは `embedded` / `file:<PATH>` / `http(s)://<URL>` のいずれかである必要がある")]
    UnknownScheme,
}

#[derive(Clone)]
pub enum IntensityStationsSource {
    Embedded,
    File(PathBuf),
    Http(Url),
}

impl FromStr for IntensityStationsSource {
    type Err = SourceParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.is_empty() || s.eq_ignore_ascii_case("embedded") {
            return Ok(Self::Embedded);
        }

        if let Some(rest) = s.strip_prefix("file:") {
            let rest = rest.strip_prefix("//").unwrap_or(rest);

            if rest.is_empty() {
                return Err(SourceParseError::EmptyPath);
            }

            return Ok(Self::File(PathBuf::from(rest)));
        }

        if s.starts_with("http://") || s.starts_with("https://") {
            return Ok(Self::Http(Url::parse(s)?));
        }

        Err(SourceParseError::UnknownScheme)
    }
}

impl fmt::Display for IntensityStationsSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Embedded => f.write_str("[embedded]"),
            Self::File(path) => write!(f, "file:{}", path.display()),
            Self::Http(url) => f.write_str(&sanitize_url(url)),
        }
    }
}

pub fn sanitize_url(url: &Url) -> String {
    let mut url = url.clone();

    let _ = url.set_username("");
    let _ = url.set_password(None);

    if url.query().is_some_and(|q| !q.is_empty()) {
        url.set_query(Some("[REDACTED]"));
    }

    url.set_fragment(None);

    url.to_string()
}
