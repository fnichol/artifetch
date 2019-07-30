use crate::Provider;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Registry {
    providers: HashMap<String, Provider>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Provider) {
        let _ = self
            .providers
            .insert(provider.domain().to_owned(), provider);
    }

    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<&Provider> {
        self.providers.get(key.as_ref())
    }

    pub fn get_mut<S: AsRef<str>>(&mut self, key: S) -> Option<&mut Provider> {
        self.providers.get_mut(key.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Provider> {
        self.providers.values()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Registry {
            providers: HashMap::new(),
        }
    }
}
