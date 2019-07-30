use crate::app;
use actix_web::web;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::io;
use std::str::FromStr;

#[derive(Debug)]
pub enum Version {
    Latest,
    Version(String),
}

impl FromStr for Version {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "latest" => Ok(Version::Latest),
            ver => Ok(Version::Version(ver.to_string())),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Version::Latest => write!(f, "latest"),
            Version::Version(version) => write!(f, "{}", version),
        }
    }
}

impl<'d> Deserialize<'d> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let s = String::deserialize(deserializer)?;
        Version::from_str(&s).map_err(de::Error::custom)
    }
}

pub trait ProviderPath {
    fn provider(&self) -> &str;
}

pub trait RepoPath: ProviderPath {
    fn owner(&self) -> &str;
    fn repo(&self) -> &str;
}

pub trait ReleasePath: RepoPath {
    fn version(&self) -> &Version;
}

pub trait TargetPath: ReleasePath {
    fn target(&self) -> &str;
}

pub trait AssetPath: TargetPath {
    fn asset(&self) -> &str;
}

#[derive(Debug, Deserialize)]
pub struct Repos {
    pub provider: String,
}

impl ProviderPath for Repos {
    fn provider(&self) -> &str {
        &self.provider
    }
}

#[derive(Debug, Deserialize)]
pub struct Releases {
    pub provider: String,
    pub owner: String,
    pub repo: String,
}

impl ProviderPath for Releases {
    fn provider(&self) -> &str {
        &self.provider
    }
}

impl RepoPath for Releases {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn repo(&self) -> &str {
        &self.repo
    }
}

#[derive(Debug, Deserialize)]
pub struct Targets {
    pub provider: String,
    pub owner: String,
    pub repo: String,
    pub version: Version,
}

impl ProviderPath for Targets {
    fn provider(&self) -> &str {
        &self.provider
    }
}

impl RepoPath for Targets {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn repo(&self) -> &str {
        &self.repo
    }
}

impl ReleasePath for Targets {
    fn version(&self) -> &Version {
        &self.version
    }
}

#[derive(Debug, Deserialize)]
pub struct Assets {
    pub provider: String,
    pub owner: String,
    pub repo: String,
    pub version: Version,
    pub target: String,
}

impl ProviderPath for Assets {
    fn provider(&self) -> &str {
        &self.provider
    }
}

impl RepoPath for Assets {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn repo(&self) -> &str {
        &self.repo
    }
}

impl ReleasePath for Assets {
    fn version(&self) -> &Version {
        &self.version
    }
}

impl TargetPath for Assets {
    fn target(&self) -> &str {
        &self.target
    }
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub provider: String,
    pub owner: String,
    pub repo: String,
    pub version: Version,
    pub target: String,
    pub asset: String,
}

impl ProviderPath for Asset {
    fn provider(&self) -> &str {
        &self.provider
    }
}

impl RepoPath for Asset {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn repo(&self) -> &str {
        &self.repo
    }
}

impl ReleasePath for Asset {
    fn version(&self) -> &Version {
        &self.version
    }
}

impl TargetPath for Asset {
    fn target(&self) -> &str {
        &self.target
    }
}

impl AssetPath for Asset {
    fn asset(&self) -> &str {
        &self.asset
    }
}

pub fn get_provider<'a, P>(
    path: &P,
    data: &'a web::Data<app::Data>,
) -> Result<&'a crate::Provider, ()>
where
    P: ProviderPath,
{
    match data.provider(path.provider()) {
        Some(p) => Ok(p),
        None => panic!("TODO: no such provider: {}", path.provider()),
    }
}

pub fn get_repo<'a, P>(path: &P, data: &'a web::Data<app::Data>) -> Result<&'a crate::Repo, ()>
where
    P: RepoPath + ProviderPath,
{
    match get_provider(path, data)?.repo(path.owner(), path.repo()) {
        Some(r) => Ok(r),
        None => panic!("TODO: no such repo: {}/{}", path.owner(), path.repo()),
    }
}

pub fn get_release<'a, P>(
    path: &P,
    data: &'a web::Data<app::Data>,
) -> Result<&'a crate::Release, ()>
where
    P: ReleasePath + RepoPath,
{
    let repo = get_repo(path, data)?;

    match match path.version() {
        Version::Latest => repo.latest_release(),
        Version::Version(version) => repo.release(version),
    } {
        Some(r) => Ok(r),
        None => panic!("TODO: no such release: {}", path.version()),
    }
}

pub fn get_target<'a, P>(path: &P, data: &'a web::Data<app::Data>) -> Result<&'a crate::Target, ()>
where
    P: TargetPath + ReleasePath,
{
    match get_release(path, data)?.target(path.target()) {
        Some(t) => Ok(t),
        None => panic!("TODO: no such target: {}", path.target()),
    }
}

pub fn get_asset<'a, P>(path: &P, data: &'a web::Data<app::Data>) -> Result<&'a crate::Asset, ()>
where
    P: AssetPath + TargetPath,
{
    match get_target(path, data)?.asset(path.asset()) {
        Some(a) => Ok(a),
        None => panic!("TODO: no such asset: {}", path.asset()),
    }
}
