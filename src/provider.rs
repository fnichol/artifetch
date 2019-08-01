use crate::Repo;
use github::GitHub;
use std::sync::Arc;

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

    pub fn repos<'a>(&'a self) -> impl Iterator<Item = Arc<Repo>> + 'a {
        match self {
            Provider::GitHub(github) => github.repos(),
        }
    }

    pub fn repo<S, T>(&self, owner: S, name: T) -> Option<Arc<Repo>>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        match self {
            Provider::GitHub(github) => github.repo(owner, name),
        }
    }

    pub fn update_repo<S, T, F>(&mut self, owner: S, name: T, update: F) -> Result<(), ()>
    where
        S: AsRef<str>,
        T: AsRef<str>,
        F: FnMut(&mut Repo),
    {
        match self {
            Provider::GitHub(github) => github.update_repo(owner, name, update),
        }
    }
}
