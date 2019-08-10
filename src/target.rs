use crate::Asset;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Target {
    name: String,
    assets: HashMap<String, Asset>,
}

impl Target {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Target {
            name: name.into(),
            assets: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.assets.values()
    }

    pub fn asset<S: AsRef<str>>(&self, name: S) -> Option<&Asset> {
        self.assets.get(name.as_ref())
    }

    pub fn set_assets<V: Into<Vec<Asset>>>(&mut self, vec: V) {
        let mut assets = HashMap::new();
        for asset in vec.into() {
            assets.insert(asset.name().to_string(), asset);
        }

        self.assets = assets;
    }

    pub fn push_asset(&mut self, asset: Asset) {
        self.assets.insert(asset.name().to_string(), asset);
    }
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Target {}

impl Hash for Target {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
