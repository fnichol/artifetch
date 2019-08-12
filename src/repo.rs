use crate::{ETag, Release};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Repo {
    owner: String,
    name: String,
    releases: HashMap<String, Release>,
    latest_id: Option<String>,
    updated: Option<Instant>,
    releases_etag: Option<ETag>,
    latest_etag: Option<ETag>,
    interval: Duration,
}

impl Repo {
    pub fn new<S, T>(owner: S, name: T) -> Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        Repo {
            owner: owner.into(),
            name: name.into(),
            releases: HashMap::new(),
            latest_id: None,
            updated: None,
            releases_etag: None,
            latest_etag: None,
            interval: Duration::from_secs(30),
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn releases(&self) -> impl Iterator<Item = &Release> {
        // TODO: sort releases by created_at
        self.releases.values()
    }

    pub fn set_releases<V: Into<Vec<Release>>>(&mut self, vec: V) {
        let mut releases = HashMap::new();
        for release in vec.into() {
            releases.insert(release.name().to_string(), release);
        }

        self.releases = releases;
    }

    pub fn release<N: AsRef<str>>(&self, name: N) -> Option<&Release> {
        self.releases.get(name.as_ref())
    }

    pub fn latest_release(&self) -> Option<&Release> {
        match self.latest_id {
            Some(ref id) => self.releases.get(id),
            None => None,
        }
    }

    pub fn set_latest_release<S: Into<String>>(&mut self, id: Option<S>) {
        self.latest_id = id.map(|id| id.into());
    }

    pub fn releases_etag(&self) -> Option<&ETag> {
        self.releases_etag.as_ref()
    }

    pub fn set_releases_etag(&mut self, etag: Option<ETag>) {
        self.releases_etag = etag;
    }

    pub fn latest_etag(&self) -> Option<&ETag> {
        self.latest_etag.as_ref()
    }

    pub fn set_latest_etag(&mut self, etag: Option<ETag>) {
        self.latest_etag = etag;
    }
}

impl PartialEq for Repo {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.name == other.name
    }
}

impl Eq for Repo {}

impl Hash for Repo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.name.hash(state);
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}
