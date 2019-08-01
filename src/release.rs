use crate::Target;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Release {
    id: String,
    targets: HashMap<String, Target>,
    updated: Option<Instant>,
}

impl Release {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Release {
            id: id.into(),
            targets: HashMap::new(),
            updated: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn targets(&self) -> impl Iterator<Item = &Target> {
        self.targets.values()
    }

    pub fn target<S: AsRef<str>>(&self, id: S) -> Option<&Target> {
        self.targets.get(id.as_ref())
    }

    pub fn set_targets<V: Into<Vec<Target>>>(&mut self, vec: V) {
        let mut targets = HashMap::new();
        for target in vec.into() {
            targets.insert(target.id().to_string(), target);
        }

        self.targets = targets;
    }
}

impl PartialEq for Release {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Release {}

impl Hash for Release {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
