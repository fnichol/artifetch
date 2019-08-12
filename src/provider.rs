use crate::Repo;
use actix_web::http::uri;
use futures::Future;
use github::GitHub;
use std::error;
use std::fmt;
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

    pub fn update_repo<O, N>(&self, owner: O, name: N) -> impl Future<Item = (), Error = Error>
    where
        O: Into<String>,
        N: Into<String>,
    {
        match self {
            Provider::GitHub(github) => github.update_repo(owner, name),
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Provider::GitHub(github) => github.fmt(f),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Client(Box<dyn error::Error + Send + Sync>),
    InvalidUri(String, uri::InvalidUri),
    RepoNotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Client(ref err) => err.fmt(f),
            Error::InvalidUri(ref uri_str, ref err) => {
                write!(f, "invalid uri {}: {}", uri_str, err)
            }
            Error::RepoNotFound => f.write_str("repository not found"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Client(ref err) => err.source(),
            Error::InvalidUri(_, ref err) => err.source(),
            Error::RepoNotFound => None,
        }
    }
}

impl From<github::client::Error> for Error {
    fn from(err: github::client::Error) -> Self {
        Error::Client(Box::new(err))
    }
}
