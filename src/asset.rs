use actix_web::http::Uri;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct Asset {
    id: String,
    download_uri: Uri,
}

impl Asset {
    pub fn new<S, U>(id: S, download_uri: U) -> Self
    where
        S: Into<String>,
        U: Into<Uri>,
    {
        Asset {
            id: id.into(),
            download_uri: download_uri.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn download_uri(&self) -> &Uri {
        &self.download_uri
    }
}

impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Asset {}

impl Hash for Asset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}
