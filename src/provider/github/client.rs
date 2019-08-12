use crate::ETag;
use futures::{
    future::{self, Either},
    Future, Stream,
};
use log::{error, warn};
use reqwest::{
    header,
    r#async::{Chunk, Client as ReqwestClient, Decoder, Response as ReqwestResponse},
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::error;
use std::fmt;
use std::str::{self, FromStr};

const DEFAULT_DOMAIN: &str = "api.github.com";

#[derive(Debug)]
pub struct Response<T> {
    etag: Option<ETag>,
    payload: T,
}

impl<T> AsRef<T> for Response<T> {
    fn as_ref(&self) -> &T {
        &self.payload
    }
}

impl<T> Response<T> {
    pub fn into_parts(self) -> (Option<ETag>, T) {
        (self.etag, self.payload)
    }
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub id: u64,
    pub tag_name: String,
    pub url: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub browser_download_url: String,
    pub content_type: String,
    pub size: u64,
    pub download_count: u64,
    pub created_at: String,
}

#[derive(Debug)]
pub struct Manifest {
    pub name: String,
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    pub fn from_bytes<N, I>(name: N, input: I) -> Result<Self, Error>
    where
        N: Into<String>,
        I: AsRef<[u8]>,
    {
        let mut entries = Vec::new();
        for line in str::from_utf8(input.as_ref())?.lines() {
            entries.push(line.parse()?);
        }

        Ok(Manifest {
            name: name.into(),
            entries,
        })
    }
}

#[derive(Debug)]
pub struct ManifestEntry {
    pub target: String,
    pub asset: String,
}

impl FromStr for ManifestEntry {
    type Err = ManifestEntryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields = s.split_ascii_whitespace().collect::<Vec<_>>();
        let num_fields = fields.len();

        if num_fields == 1 {
            Err(ManifestEntryParseError(
                "missing whitespace delimiter between fields",
            ))
        } else if num_fields > 2 {
            Err(ManifestEntryParseError("more than two fields"))
        } else if num_fields != 2 {
            unreachable!("invalid number of fields");
        } else {
            Ok(ManifestEntry {
                target: fields[0].to_string(),
                asset: fields[1].to_string(),
            })
        }
    }
}

pub struct Client {
    inner: HttpClient,
}

impl Client {
    pub fn build<O: AsRef<str>>(oauth_token: O) -> Result<Self, Error> {
        Ok(Self {
            inner: HttpClient::build(oauth_token)?,
        })
    }

    pub fn build_for_enterprise<D, O>(domain: D, oauth_token: O) -> Result<Self, Error>
    where
        D: AsRef<str>,
        O: AsRef<str>,
    {
        Ok(Self {
            inner: HttpClient::build_for_enterprise(domain, oauth_token)?,
        })
    }

    pub fn releases<O, N>(
        &self,
        owner: O,
        name: N,
        etag: Option<&ETag>,
    ) -> impl Future<Item = Option<Response<Vec<Release>>>, Error = Error>
    where
        O: AsRef<str>,
        N: AsRef<str>,
    {
        self.inner.get(
            format!("/repos/{}/{}/releases", owner.as_ref(), name.as_ref()),
            None::<&str>,
            etag,
        )
    }

    pub fn latest_release<O, N>(
        &self,
        owner: O,
        name: N,
        etag: Option<&ETag>,
    ) -> impl Future<Item = Option<Response<Release>>, Error = Error>
    where
        O: AsRef<str>,
        N: AsRef<str>,
    {
        self.inner.get(
            format!(
                "/repos/{}/{}/releases/latest",
                owner.as_ref(),
                name.as_ref()
            ),
            None::<&str>,
            etag,
        )
    }

    pub fn manifest<O, N, A>(
        &self,
        owner: O,
        name: N,
        asset_id: u64,
        asset_name: A,
    ) -> impl Future<Item = Manifest, Error = Error>
    where
        O: AsRef<str>,
        N: AsRef<str>,
        A: Into<String>,
    {
        self.inner
            .get_body(
                format!(
                    "/repos/{}/{}/releases/assets/{}",
                    owner.as_ref(),
                    name.as_ref(),
                    asset_id
                ),
                None::<&str>,
            )
            .and_then(|bytes| Manifest::from_bytes(asset_name, bytes))
    }
}

struct HttpClient {
    inner: ReqwestClient,
    domain: String,
}

impl HttpClient {
    fn build<O: AsRef<str>>(oauth_token: O) -> Result<Self, Error> {
        Ok(Self {
            inner: reqwest_client(oauth_token)?,
            domain: DEFAULT_DOMAIN.to_string(),
        })
    }

    fn build_for_enterprise<D, O>(domain: D, oauth_token: O) -> Result<Self, Error>
    where
        D: AsRef<str>,
        O: AsRef<str>,
    {
        Ok(Self {
            inner: reqwest_client(oauth_token)?,
            domain: format!("{}/api/v3", domain.as_ref()),
        })
    }

    fn get<P, Q, T>(
        &self,
        path: P,
        query: Option<Q>,
        etag: Option<&ETag>,
    ) -> impl Future<Item = Option<Response<T>>, Error = Error>
    where
        P: AsRef<str>,
        Q: AsRef<str>,
        T: DeserializeOwned,
    {
        let mut req = self.inner.get(&self.url(path, query));
        if let Some(etag) = etag {
            let val = match header::HeaderValue::from_str(etag.as_ref()) {
                Ok(val) => val,
                Err(err) => return Either::A(future::err(Error::InvalidHeaderValue("etag", err))),
            };
            req = req.header(header::IF_NONE_MATCH, val);
        }

        Either::B(req.send().map_err(Error::Request).and_then(|mut response| {
            if response.status() == StatusCode::NOT_MODIFIED {
                Either::A(future::ok(None))
            } else if response.status().is_success() {
                let etag = response_etag(&response);

                Either::B(Either::A(
                    response
                        .json()
                        .map_err(Error::Deserialize)
                        .map(|t| Some(Response { etag, payload: t })),
                ))
            } else if response.status() == StatusCode::NOT_FOUND {
                Either::B(Either::B(Either::A(future::err(Error::NotFound))))
            } else {
                Either::B(Either::B(Either::B(
                    response
                        .json()
                        .map_err(Error::Deserialize)
                        .and_then(|err| future::err(Error::Api(err))),
                )))
            }
        }))
    }

    fn get_body<P, Q>(&self, path: P, query: Option<Q>) -> impl Future<Item = Chunk, Error = Error>
    where
        P: AsRef<str>,
        Q: AsRef<str>,
    {
        self.inner
            .get(&self.url(path, query))
            .header(
                header::ACCEPT,
                header::HeaderValue::from_static("application/octet-stream"),
            )
            .send()
            .map_err(Error::Request)
            .and_then(|mut response| {
                if response.status().is_success() {
                    let body = std::mem::replace(response.body_mut(), Decoder::empty());
                    Either::A(body.concat2().map_err(Error::Response))
                } else {
                    Either::B(
                        response
                            .json()
                            .map_err(Error::Deserialize)
                            .and_then(|err| Err(Error::Api(err))),
                    )
                }
            })
    }

    fn url<P, Q>(&self, path: P, query: Option<Q>) -> String
    where
        P: AsRef<str>,
        Q: AsRef<str>,
    {
        let mut url = format!("https://{}{}", &self.domain, path.as_ref());
        if let Some(query) = query {
            url.push_str("?");
            url.push_str(query.as_ref());
        }

        url
    }
}

#[derive(Debug)]
pub enum Error {
    Api(RequestError),
    Builder(reqwest::Error),
    Deserialize(reqwest::Error),
    InvalidHeaderValue(&'static str, reqwest::header::InvalidHeaderValue),
    Manifest(ManifestEntryParseError),
    NotFound,
    Request(reqwest::Error),
    Response(reqwest::Error),
    Utf8(str::Utf8Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Api(ref err) => err.fmt(f),
            Error::Builder(ref err) => err.fmt(f),
            Error::Deserialize(ref err) => err.fmt(f),
            Error::InvalidHeaderValue(ref name, ref err) => {
                write!(f, "valid header value for {}: {}", name, err)
            }
            Error::Manifest(ref err) => err.fmt(f),
            Error::NotFound => f.write_str("not found"),
            Error::Request(ref err) => err.fmt(f),
            Error::Response(ref err) => err.fmt(f),
            Error::Utf8(ref err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Api(ref err) => err.source(),
            Error::Builder(ref err) => err.source(),
            Error::Deserialize(ref err) => err.source(),
            Error::InvalidHeaderValue(_, ref err) => err.source(),
            Error::Manifest(ref err) => err.source(),
            Error::NotFound => None,
            Error::Request(ref err) => err.source(),
            Error::Response(ref err) => err.source(),
            Error::Utf8(ref err) => err.source(),
        }
    }
}

impl From<str::Utf8Error> for Error {
    fn from(err: str::Utf8Error) -> Self {
        Error::Utf8(err)
    }
}

impl From<ManifestEntryParseError> for Error {
    fn from(err: ManifestEntryParseError) -> Self {
        Error::Manifest(err)
    }
}

#[derive(Debug)]
pub struct ManifestEntryParseError(&'static str);

impl fmt::Display for ManifestEntryParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl error::Error for ManifestEntryParseError {}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestError {
    pub success: bool,
    pub message: String,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "request error: {} (success={})",
            self.message, self.success
        )
    }
}

impl error::Error for RequestError {}

fn reqwest_client<O: AsRef<str>>(oauth_token: O) -> Result<ReqwestClient, Error> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/vnd.github.v3+json"),
    );
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("token {}", oauth_token.as_ref()))
            .map_err(|err| Error::InvalidHeaderValue("authorization", err))?,
    );

    ReqwestClient::builder()
        .default_headers(headers)
        .build()
        .map_err(Error::Builder)
}

fn response_etag(response: &ReqwestResponse) -> Option<ETag> {
    match response.headers().get(header::ETAG) {
        Some(header) => match header.to_str() {
            Ok(s) => match s.parse() {
                Ok(etag) => Some(etag),
                Err(err) => {
                    error!("etag header could not be parsed; err={}", err);
                    None
                }
            },
            Err(err) => {
                error!("etag header was not utf8 clean; err={}", err);
                None
            }
        },
        None => {
            warn!("etag header not found in response and was expected");
            None
        }
    }
}
