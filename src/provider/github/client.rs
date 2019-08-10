use crate::ETag;
use futures::{
    future::{self, Either},
    Future, Stream,
};
use reqwest::{
    header,
    r#async::{Chunk, Client as ReqwestClient, Decoder, Response as ReqwestResponse},
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;
use std::str::{self, FromStr};

const DEFAULT_DOMAIN: &str = "api.github.com";

#[derive(Debug)]
pub enum Error {
    Api(RequestError),
    Request(reqwest::Error),
    Deserialize(reqwest::Error),
    NotFound,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestError {
    pub success: bool,
    pub message: String,
}

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
    pub fn from_bytes<N, I>(name: N, input: I) -> Result<Self, io::Error>
    where
        N: Into<String>,
        I: AsRef<[u8]>,
    {
        let mut entries = Vec::new();
        for line in str::from_utf8(input.as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?
            .lines()
        {
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
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields = s.split_ascii_whitespace().collect::<Vec<_>>();
        let num_fields = fields.len();

        if num_fields == 1 {
            panic!("TODO: missing whitespace delimiter");
        } else if num_fields > 2 {
            panic!("TODO: too many fields");
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
    pub fn new<O: AsRef<str>>(oauth_token: O) -> Self {
        Self {
            inner: HttpClient::new(oauth_token),
        }
    }

    pub fn for_enterprise<D, O>(domain: D, oauth_token: O) -> Self
    where
        D: AsRef<str>,
        O: AsRef<str>,
    {
        Self {
            inner: HttpClient::for_enterprise(domain, oauth_token),
        }
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
            .and_then(|bytes| {
                Manifest::from_bytes(asset_name, bytes)
                    .map_err(|err| panic!("TODO handle conversion to Manifest; err={:?}", err))
            })
    }
}

struct HttpClient {
    inner: ReqwestClient,
    domain: String,
}

impl HttpClient {
    fn new<O: AsRef<str>>(oauth_token: O) -> Self {
        Self {
            inner: reqwest_client(oauth_token),
            domain: DEFAULT_DOMAIN.to_string(),
        }
    }

    fn for_enterprise<D, O>(domain: D, oauth_token: O) -> Self
    where
        D: AsRef<str>,
        O: AsRef<str>,
    {
        Self {
            inner: reqwest_client(oauth_token),
            domain: format!("{}/api/v3", domain.as_ref()),
        }
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
            req = req.header(
                header::IF_NONE_MATCH,
                header::HeaderValue::from_str(etag.as_ref())
                    .expect("TODO: handle InvalidHeaderValue"),
            );
        }

        req.send().map_err(Error::Request).and_then(|mut response| {
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
        })
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
                    Either::A(body.concat2().map_err(|err| panic!("TODO: err={:?}", err)))
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

fn reqwest_client<O: AsRef<str>>(oauth_token: O) -> ReqwestClient {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/vnd.github.v3+json"),
    );
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&format!("token {}", oauth_token.as_ref()))
            .expect("TODO: handle InvalidHeaderValue"),
    );

    ReqwestClient::builder()
        .default_headers(headers)
        .build()
        .expect("TODO: handle TLS backend failure")
}

fn response_etag(response: &ReqwestResponse) -> Option<ETag> {
    response.headers().get(header::ETAG).map(|header| {
        header
            .to_str()
            .expect("TODO: header be utf8 clean")
            .parse()
            .expect("TODO: HeaderValue should parse into ETag")
    })
}
