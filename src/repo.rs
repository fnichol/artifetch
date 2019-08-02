use crate::Release;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Repo {
    owner: String,
    name: String,
    releases: HashMap<String, Release>,
    latest_id: Option<String>,
    updated: Option<Instant>,
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

    pub fn release<S: AsRef<str>>(&self, id: S) -> Option<&Release> {
        self.releases.get(id.as_ref())
    }

    pub fn latest_release(&self) -> Option<&Release> {
        match self.latest_id {
            Some(ref id) => self.releases.get(id),
            None => None,
        }
    }

    pub fn set_releases<V: Into<Vec<Release>>>(&mut self, vec: V) {
        let mut releases = HashMap::new();
        for release in vec.into() {
            releases.insert(release.id().to_string(), release);
        }

        self.releases = releases;
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

impl FromStr for Repo {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('/').collect();

        if parts.len() == 1 {
            panic!("TODO: repo missing /");
        } else if parts.len() > 2 {
            panic!("TODO: repo with too many /");
        } else if parts.len() != 2 {
            unreachable!("illegal repo string format");
        }

        Ok(Repo::new(parts[0], parts[1]))
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}
