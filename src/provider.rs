use crate::Repo;
use github::GitHub;

pub mod github;

#[derive(Debug)]
pub enum Provider {
    GitHub(GitHub),
}

impl Provider {
    pub fn domain(&self) -> &str {
        match self {
            Provider::GitHub(github) => github.domain(),
        }
    }

    pub fn repos(&self) -> impl Iterator<Item = &Repo> {
        match self {
            Provider::GitHub(github) => github.repos(),
        }
    }

    pub fn repo<S, T>(&self, owner: S, name: T) -> Option<&Repo>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        match self {
            Provider::GitHub(github) => github.repo(owner, name),
        }
    }

    pub fn repo_mut<S, T>(&mut self, owner: S, name: T) -> Option<&mut Repo>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        match self {
            Provider::GitHub(github) => github.repo_mut(owner, name),
        }
    }
}
