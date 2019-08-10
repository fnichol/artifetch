use crate::Repo;
use futures::Future;
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

    pub fn update_repo<O, N>(&self, owner: O, name: N) -> impl Future<Item = (), Error = ()>
    where
        O: Into<String>,
        N: Into<String>,
    {
        match self {
            Provider::GitHub(github) => github.update_repo(owner, name),
        }
    }
}
