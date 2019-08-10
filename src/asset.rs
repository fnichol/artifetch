use actix_web::http::Uri;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Asset {
    name: String,
    download_uri: Uri,
}

impl Asset {
    pub fn new<S, U>(name: S, download_uri: U) -> Self
    where
        S: Into<String>,
        U: Into<Uri>,
    {
        Asset {
            name: name.into(),
            download_uri: download_uri.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn download_uri(&self) -> &Uri {
        &self.download_uri
    }
}
