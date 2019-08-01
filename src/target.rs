use crate::Asset;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Target {
    id: String,
    assets: HashMap<String, Asset>,
}

impl Target {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Target {
            id: id.into(),
            assets: HashMap::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.values()
    }

    pub fn asset<S: AsRef<str>>(&self, id: S) -> Option<&Asset> {
        self.assets.get(id.as_ref())
    }

    pub fn set_assets<V: Into<Vec<Asset>>>(&mut self, vec: V) {
        let mut assets = HashMap::new();
        for asset in vec.into() {
            assets.insert(asset.id().to_string(), asset);
        }

        self.assets = assets;
    }
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Target {}

impl Hash for Target {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
