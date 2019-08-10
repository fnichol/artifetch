use std::fmt;
use std::io;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ETag(String);

impl ETag {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ETag {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl AsRef<str> for ETag {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for ETag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
