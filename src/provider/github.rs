use crate::Repo;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GitHub {
    domain: String,
    repos: HashMap<String, HashMap<String, Repo>>,
}

impl GitHub {
    pub fn new<S, R>(domain: S, iter: R) -> Self
    where
        S: Into<String>,
        R: IntoIterator<Item = Repo>,
    {
        let mut repos = HashMap::new();
        for repo in iter {
            repos
                .entry(repo.owner().to_string())
                .or_insert_with(HashMap::new)
                .insert(repo.name().to_string(), repo);
        }

        GitHub {
            domain: domain.into(),
            repos,
        }
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn repos(&self) -> impl Iterator<Item = &Repo> {
        self.repos.values().map(|i| i.values()).flatten()
    }

    pub fn repo<S, T>(&self, owner: S, name: T) -> Option<&Repo>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        self.repos
            .get(owner.as_ref())
            .and_then(|o| o.get(name.as_ref()))
    }

    pub fn repo_mut<S, T>(&mut self, owner: S, name: T) -> Option<&mut Repo>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        self.repos
            .get_mut(owner.as_ref())
            .and_then(|o| o.get_mut(name.as_ref()))
    }
}
