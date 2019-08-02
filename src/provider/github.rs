use crate::Repo;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct GitHub {
    domain: String,
    repos: HashMap<String, HashMap<String, RwLock<Arc<Repo>>>>,
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
                .insert(repo.name().to_string(), RwLock::new(Arc::new(repo)));
        }

        GitHub {
            domain: domain.into(),
            repos,
        }
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

    pub fn update_repo<S, T, F>(&self, owner: S, name: T, update: F) -> Result<(), ()>
    where
        S: AsRef<str>,
        T: AsRef<str>,
        F: FnOnce(&mut Repo),
    {
        let mut repo = self
            .repos
            .get(owner.as_ref())
            .and_then(|o| o.get(name.as_ref()))
            .map(|r| r.write().expect("lock poisoned"))
            .ok_or_else(|| panic!("TODO: no such repo: {}/{}", owner.as_ref(), name.as_ref()))?;

        update(Arc::make_mut(&mut repo));

        Ok(())
    }
}
