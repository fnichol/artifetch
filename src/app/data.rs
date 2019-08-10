use crate::{Provider, Registry};

#[derive(Debug)]
pub struct Data {
    registry: Registry,
}

impl Data {
    pub fn new(registry: Registry) -> Self {
        Self { registry }
    }

    pub fn provider<S: AsRef<str>>(&self, key: S) -> Option<&Provider> {
        self.registry.get(key.as_ref())
    }

    pub fn providers(&self) -> impl Iterator<Item = &Provider> {
        self.registry.iter()
    }
}
