use super::Error;
use crate::{Asset, ETag, Release, Repo, Target};
use actix_web::http::{HttpTryFrom, Uri};
use futures::{
    future::{self, Either},
    Future,
};
use log::info;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

pub mod client;

const NO_ETAG: &str = "<none>";
const MANIFEST_EXT: &str = ".manifest.txt";

type RepoMap = Arc<HashMap<String, HashMap<String, RwLock<Arc<Repo>>>>>;

pub struct GitHub {
    domain: String,
    client: Arc<client::Client>,
    repos: RepoMap,
}

impl fmt::Display for GitHub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "provider::github({})", &self.domain)
    }
}

impl fmt::Debug for GitHub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("GitHub")
            .field("domain", &self.domain)
            .field("repos", &self.repos)
            .finish()
    }
}

impl GitHub {
    pub fn build<S, O, R>(domain: S, oauth_token: O, iter: R) -> Result<Self, Error>
    where
        S: Into<String>,
        O: AsRef<str>,
        R: IntoIterator<Item = Repo>,
    {
        let domain = domain.into();
        let client = match domain.as_str() {
            "github.com" => Arc::new(client::Client::build(oauth_token)?),
            enterprise => Arc::new(client::Client::build_for_enterprise(
                enterprise,
                oauth_token,
            )?),
        };
        let mut repos = HashMap::new();
        for repo in iter {
            repos
                .entry(repo.owner().to_string())
                .or_insert_with(HashMap::new)
                .insert(repo.name().to_string(), RwLock::new(Arc::new(repo)));
        }
        let repos = Arc::new(repos);

        Ok(GitHub {
            domain,
            client,
            repos,
        })
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn repos<'a>(&'a self) -> impl Iterator<Item = Arc<Repo>> + 'a {
        self.repos
            .values()
            .map(|i| i.values().map(|r| r.read().expect("lock poisoned").clone()))
            .flatten()
    }

    pub fn repo<S, T>(&self, owner: S, name: T) -> Option<Arc<Repo>>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        self.repos
            .get(owner.as_ref())
            .and_then(|o| o.get(name.as_ref()))
            .map(|r| r.read().expect("lock poisoned").clone())
    }

    pub fn update_repo<O, N>(&self, owner: O, name: N) -> impl Future<Item = (), Error = Error>
    where
        O: Into<String>,
        N: Into<String>,
    {
        let owner: Arc<str> = owner.into().into();
        let name: Arc<str> = name.into().into();
        let domain: Arc<str> = self.domain.clone().into();

        let (releases_etag, latest_etag) = {
            let repo = match self.repo(&owner, &name) {
                Some(repo) => repo,
                None => return Either::A(future::err(Error::RepoNotFound)),
            };

            (repo.releases_etag().cloned(), repo.latest_etag().cloned())
        };

        Either::B(
            update_releases(
                self.client.clone(),
                self.repos.clone(),
                domain.clone(),
                owner.clone(),
                name.clone(),
                releases_etag,
            )
            .join(update_latest(
                self.client.clone(),
                self.repos.clone(),
                domain,
                owner,
                name,
                latest_etag,
            ))
            .and_then(|_| future::ok(())),
        )
    }
}

fn update_releases(
    client: Arc<client::Client>,
    repos: RepoMap,
    domain: Arc<str>,
    owner: Arc<str>,
    name: Arc<str>,
    etag: Option<ETag>,
) -> impl Future<Item = (), Error = Error> {
    client
        .releases(owner.clone(), name.clone(), etag.as_ref())
        .map_err(|err| Error::Client(Box::new(err)))
        .and_then(move |response| match response {
            None => {
                info!(
                    "releases not modified; domain={}, repo={}/{}, etag={}",
                    &domain,
                    &owner,
                    &name,
                    etag.as_ref().map(|e| e.as_ref()).unwrap_or_else(|| NO_ETAG)
                );

                Either::A(future::ok(()))
            }
            Some(response) => {
                let (next_etag, releases) = response.into_parts();

                Either::B(
                    process_releases(client, releases, owner.clone(), name.clone()).and_then(
                        move |releases| match repo_mut(repos, &owner, &name, |repo| {
                            repo.set_releases_etag(next_etag.as_ref().cloned());
                            repo.set_releases(releases);
                        }) {
                            Ok(_) => {
                                info!(
                                    "releases updated; domain={}, repo={}/{}, next_etag={}",
                                    &domain,
                                    &owner,
                                    &name,
                                    next_etag
                                        .as_ref()
                                        .map(|e| e.as_ref())
                                        .unwrap_or_else(|| NO_ETAG)
                                );

                                future::ok(())
                            }
                            Err(err) => future::err(err),
                        },
                    ),
                )
            }
        })
}

fn update_latest(
    client: Arc<client::Client>,
    repos: RepoMap,
    domain: Arc<str>,
    owner: Arc<str>,
    name: Arc<str>,
    etag: Option<ETag>,
) -> impl Future<Item = (), Error = Error> {
    client
        .latest_release(owner.clone(), name.clone(), etag.as_ref())
        .map_err(|err| match err {
            client::Error::NotFound => Error::LatestNotFound,
            err => Error::Client(Box::new(err)),
        })
        .and_then(move |response| match response {
            None => {
                info!(
                    "latest release not modified; domain={}, repo={}/{}, etag={}",
                    &domain,
                    &owner,
                    &name,
                    etag.as_ref().map(|e| e.as_ref()).unwrap_or_else(|| NO_ETAG)
                );

                future::ok(())
            }
            Some(response) => {
                let (next_etag, latest) = response.into_parts();

                match repo_mut(repos, &owner, &name, |repo| {
                    repo.set_latest_etag(next_etag.as_ref().cloned());
                    repo.set_latest_release(Some(latest.tag_name));
                }) {
                    Ok(_) => {
                        info!(
                            "latest release updated; domain={}, repo={}/{}, next_etag={}",
                            &domain,
                            &owner,
                            &name,
                            next_etag
                                .as_ref()
                                .map(|e| e.as_ref())
                                .unwrap_or_else(|| NO_ETAG)
                        );

                        future::ok(())
                    }
                    Err(err) => future::err(err),
                }
            }
        })
}

fn process_releases(
    client: Arc<client::Client>,
    releases: Vec<client::Release>,
    owner: Arc<str>,
    name: Arc<str>,
) -> impl Future<Item = Vec<Release>, Error = Error> {
    let filtered_releases = releases
        .into_iter()
        .filter(|rel| !rel.draft && !rel.prerelease)
        .collect::<Vec<_>>();

    let mut all_manifests = Vec::new();
    for release in &filtered_releases {
        all_manifests.push(future::join_all(
            release
                .assets
                .iter()
                .filter(|asset| asset.name.ends_with(MANIFEST_EXT))
                .map(|asset| {
                    client
                        .manifest(
                            owner.clone(),
                            name.clone(),
                            asset.id,
                            asset.name.trim_end_matches(MANIFEST_EXT).to_string(),
                        )
                        .map_err(|err| Error::Client(Box::new(err)))
                })
                .collect::<Vec<_>>(),
        ));
    }

    future::join_all(all_manifests)
        .and_then(move |all_manifests| convert_releases(filtered_releases, all_manifests))
}

fn convert_releases(
    client_releases: Vec<client::Release>,
    client_all_manifests: Vec<Vec<client::Manifest>>,
) -> Result<Vec<Release>, Error> {
    let mut releases = Vec::new();
    for (release, manifests) in client_releases.into_iter().zip(client_all_manifests) {
        releases.push(convert_release(release, manifests)?);
    }

    Ok(releases)
}

fn convert_release(
    release: client::Release,
    manifests: Vec<client::Manifest>,
) -> Result<Release, Error> {
    let mut targets = HashMap::new();
    for manifest in manifests {
        for entry in manifest.entries {
            let (entry_target, entry_asset) = (entry.target, entry.asset);

            let target = targets
                .entry(entry_target.clone())
                .or_insert_with(|| Target::new(entry_target));
            target.push_asset(Asset::new(
                manifest.name.clone(),
                uri_for_asset(&entry_asset, &release.assets)?,
            ));
        }
    }

    let mut converted = Release::from(release);
    converted.set_targets(
        targets
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
    );

    Ok(converted)
}

fn repo_mut<F>(repos: RepoMap, owner: &str, name: &str, update: F) -> Result<(), Error>
where
    F: FnOnce(&mut Repo),
{
    let mut repo = repos
        .get(owner)
        .and_then(|o| o.get(name))
        .map(|r| r.write().expect("lock poisoned"))
        .ok_or(Error::RepoNotFound)?;

    update(Arc::make_mut(&mut repo));

    Ok(())
}

fn uri_for_asset(gh_name: &str, assets: &[client::Asset]) -> Result<Uri, Error> {
    let uri_str = assets
        .iter()
        .find(|a| a.name == gh_name)
        .map(|a| &a.browser_download_url)
        .ok_or_else(|| client::Error::MissingResponseField("browser_download_url"))?;

    Uri::try_from(uri_str).map_err(|err| Error::InvalidUri(uri_str.to_string(), err))
}

impl From<client::Release> for Release {
    fn from(cr: client::Release) -> Self {
        Release::new(cr.id, cr.tag_name)
    }
}
