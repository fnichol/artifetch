use crate::app;
use actix_web::web;
use owning_ref::OwningRef;
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::io;
use std::str::FromStr;
use std::sync::Arc;

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

pub type ReleaseRef = OwningRef<Arc<crate::Repo>, crate::Release>;
pub type TargetRef = OwningRef<ReleaseRef, crate::Target>;
pub type AssetRef = OwningRef<TargetRef, crate::Asset>;

pub fn get_provider<'a, P>(
    path: &P,
    data: &'a web::Data<app::Data>,
) -> Result<&'a crate::Provider, ()>
where
    P: ProviderPath,
{
    data.provider(path.provider())
        .ok_or_else(|| panic!("TODO: no such provider: {}", path.provider()))
}

pub fn get_repo<P>(path: &P, data: &web::Data<app::Data>) -> Result<Arc<crate::Repo>, ()>
where
    P: RepoPath + ProviderPath,
{
    get_provider(path, data)?
        .repo(path.owner(), path.repo())
        .ok_or_else(|| panic!("TODO: no such repo: {}/{}", path.owner(), path.repo()))
}

pub fn get_release<P>(path: &P, data: &web::Data<app::Data>) -> Result<ReleaseRef, ()>
where
    P: ReleasePath + RepoPath,
{
    OwningRef::new(get_repo(path, data)?).try_map(|repo| {
        match path.version() {
            Version::Latest => repo.latest_release(),
            Version::Version(version) => repo.release(version),
        }
        .ok_or_else(|| panic!("TODO: no such release: {}", path.version()))
    })
}

pub fn get_target<P>(path: &P, data: &web::Data<app::Data>) -> Result<TargetRef, ()>
where
    P: TargetPath + ReleasePath,
{
    OwningRef::new(get_release(path, data)?).try_map(|release| {
        release
            .target(path.target())
            .ok_or_else(|| panic!("TODO: no such target: {}", path.target()))
    })
}

pub fn get_asset<P>(path: &P, data: &web::Data<app::Data>) -> Result<AssetRef, ()>
where
    P: AssetPath + TargetPath,
{
    OwningRef::new(get_target(path, data)?).try_map(|target| {
        target
            .asset(path.asset())
            .ok_or_else(|| panic!("TODO: no such asset: {}", path.asset()))
    })
}
