use crate::{Asset, ETag, Release, Repo, Target};
use actix_web::http::{HttpTryFrom, Uri};
use futures::{
    future::{self, Either},
    Future,
};
use log::{info, warn};
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

impl fmt::Debug for GitHub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("GitHub")
            .field("domain", &self.domain)
            .field("repos", &self.repos)
            .finish()
    }
}

impl GitHub {
    pub fn build<S, O, R>(domain: S, oauth_token: O, iter: R) -> Result<Self, client::Error>
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

    pub fn update_repo<O, N>(&self, owner: O, name: N) -> impl Future<Item = (), Error = ()>
    where
        O: Into<String>,
        N: Into<String>,
    {
        let owner = owner.into();
        let name = name.into();
        let (releases_etag, latest_etag) = {
            let repo = self.repo(owner.as_str(), name.as_str()).unwrap_or_else(|| {
                panic!("TODO: no such repo: {}/{}", owner.as_str(), name.as_str())
            });

            (repo.releases_etag().cloned(), repo.latest_etag().cloned())
        };
        let domain = Arc::new(self.domain.clone());

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
            domain.clone(),
            owner,
            name,
            latest_etag,
        ))
        .map_err(|err| panic!("TODO: ah crap; err={:?}", err))
        .and_then(|_| Ok(()))
    }
}

fn repo_mut<S, T, F>(repos: RepoMap, owner: S, name: T, update: F) -> Result<(), ()>
where
    S: AsRef<str>,
    T: AsRef<str>,
    F: FnOnce(&mut Repo),
{
    let mut repo = repos
        .get(owner.as_ref())
        .and_then(|o| o.get(name.as_ref()))
        .map(|r| r.write().expect("lock poisoned"))
        .ok_or_else(|| panic!("TODO: no such repo: {}/{}", owner.as_ref(), name.as_ref()))?;

    update(Arc::make_mut(&mut repo));

    Ok(())
}

fn update_releases(
    client: Arc<client::Client>,
    repos: RepoMap,
    domain: Arc<String>,
    owner: String,
    name: String,
    etag: Option<ETag>,
) -> impl Future<Item = (), Error = client::Error> {
    let (domain1, domain2) = (domain.clone(), domain.clone());

    let repo = Arc::new(format!("{}/{}", &owner, &name));
    let (repo_str1, repo_str2) = (repo.clone(), repo.clone());

    client
        .releases(owner.clone(), name.clone(), etag.as_ref())
        .or_else(move |err| match err {
            client::Error::NotFound => {
                warn!("no repo found; domain={}, repo={}", domain1, repo_str1);
                Either::A(future::ok(None))
            }
            err => Either::B(future::err(err)),
        })
        .and_then(move |response| match response {
            None => {
                info!(
                    "releases not modified; domain={}, repo={}, etag={}",
                    domain2,
                    repo_str2,
                    etag.as_ref().map(|e| e.as_ref()).unwrap_or_else(|| NO_ETAG)
                );

                Either::A(future::ok(()))
            }
            Some(response) => {
                let (next_etag, releases) = response.into_parts();

                Either::B(
                    process_releases(client.clone(), releases, owner.clone(), name.clone())
                        .and_then(move |releases| {
                            repo_mut(repos, owner, name, |repo| {
                                repo.set_releases_etag(next_etag.as_ref().cloned());
                                repo.set_releases(releases);
                            })
                            .expect("TODO: handle this");

                            info!(
                                "releases updated; domain={}, repo={}, next_etag={}",
                                domain2,
                                repo_str2,
                                next_etag
                                    .as_ref()
                                    .map(|e| e.as_ref())
                                    .unwrap_or_else(|| NO_ETAG)
                            );

                            future::ok(())
                        }),
                )
            }
        })
}

fn update_latest(
    client: Arc<client::Client>,
    repos: RepoMap,
    domain: Arc<String>,
    owner: String,
    name: String,
    etag: Option<ETag>,
) -> impl Future<Item = (), Error = client::Error> {
    let (domain1, domain2) = (domain.clone(), domain.clone());

    let repo_str = Arc::new(format!("{}/{}", &owner, &name));
    let (repo_str1, repo_str2) = (repo_str.clone(), repo_str.clone());

    client
        .latest_release(owner.clone(), name.clone(), etag.as_ref())
        .or_else(move |err| match err {
            client::Error::NotFound => {
                warn!(
                    "no latest release found; domain={}, repo={}",
                    domain1, repo_str1
                );
                Either::A(future::ok(None))
            }
            err => Either::B(future::err(err)),
        })
        .and_then(move |response| match response {
            None => {
                info!(
                    "latest release not modified; domain={}, repo={}, etag={}",
                    domain2,
                    repo_str2,
                    etag.as_ref().map(|e| e.as_ref()).unwrap_or_else(|| NO_ETAG)
                );

                Ok(())
            }
            Some(response) => {
                let (next_etag, latest) = response.into_parts();

                repo_mut(repos, owner, name, |repo| {
                    repo.set_latest_etag(next_etag.as_ref().cloned());
                    repo.set_latest_release(Some(latest.tag_name));
                })
                .expect("TODO: handle this");

                info!(
                    "latest release updated; domain={}, repo={}, next_etag={}",
                    domain2,
                    repo_str2,
                    next_etag
                        .as_ref()
                        .map(|e| e.as_ref())
                        .unwrap_or_else(|| NO_ETAG)
                );

                Ok(())
            }
        })
}

fn process_releases(
    client: Arc<client::Client>,
    releases: Vec<client::Release>,
    owner: String,
    name: String,
) -> impl Future<Item = Vec<Release>, Error = client::Error> {
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
                    client.manifest(
                        owner.clone(),
                        name.clone(),
                        asset.id,
                        asset.name.trim_end_matches(MANIFEST_EXT).to_string(),
                    )
                })
                .collect::<Vec<_>>(),
        ));
    }

    future::join_all(all_manifests).and_then(move |all_manifests| {
        let mut converted_releases = Vec::new();

        for (release, manifests) in filtered_releases.into_iter().zip(all_manifests) {
            let mut targets = HashMap::new();

            for manifest in manifests {
                for entry in manifest.entries {
                    let target = targets
                        .entry(entry.target.clone())
                        .or_insert_with(|| Target::new(entry.target.clone()));
                    target.push_asset(Asset::new(
                        manifest.name.clone(),
                        uri_for_asset(&entry.asset, &release.assets),
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
            converted_releases.push(converted);
        }

        future::ok(converted_releases)
    })
}

fn uri_for_asset(gh_name: &str, assets: &[client::Asset]) -> Uri {
    let uri_str = assets
        .iter()
        .find(|a| a.name == gh_name)
        .map(|a| &a.browser_download_url)
        .expect("TODO: asset should exist");

    Uri::try_from(uri_str).expect("TODO: URI should parse")
}

impl From<client::Release> for Release {
    fn from(cr: client::Release) -> Self {
        Release::new(cr.id, cr.tag_name)
    }
}
