use crate::app::{Data, Error};
use crate::env;
use crate::{Provider, Registry};
use serde::de::{self, IntoDeserializer};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::marker::PhantomData;
use std::net::{self, SocketAddr};
use std::str::FromStr;

pub fn config() -> Result<Config, Error> {
    // TODO: remove stub
    use serde_json::json;

    let json = json!({
        "bind_addr": "127.0.0.1:8080",
        "registry": {
            "github.com": {
                "oauth_token": "$GITHUB_TOKEN",
                "repos": [
                    "fnichol/testr",
                ],
            },
        }
    })
    .to_string();

    Config::from_json_str(&json)
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_bind_addr", deserialize_with = "de_bind_addr")]
    pub bind_addr: SocketAddr,
    pub registry: HashMap<String, RegistryConfig>,
}

impl Config {
    pub fn from_json_str(s: &str) -> Result<Self, Error> {
        serde_json::from_str(&s).map_err(|err| Error::ConfigLoad(Box::new(err)))
    }
}

impl TryFrom<Config> for Data {
    type Error = Error;

    fn try_from(config: Config) -> Result<Self, Self::Error> {
        let mut registry = Registry::new();
        for (name, entry) in config.registry {
            match entry {
                RegistryConfig::GitHub { oauth_token, repos } => {
                    use crate::provider::github::GitHub;

                    registry.register(Provider::GitHub(GitHub::build(
                        name,
                        oauth_token,
                        repos.into_iter().map(crate::Repo::from).collect::<Vec<_>>(),
                    )?))
                }
            }
        }

        Ok(Self::new(registry))
    }
}

#[derive(Debug)]
pub enum RegistryConfig {
    GitHub {
        repos: Vec<Repo>,
        oauth_token: String,
    },
}

#[derive(Debug)]
pub struct Repo(String, String);

impl FromStr for Repo {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields = s.split('/').collect::<Vec<_>>();
        let num_fields = fields.len();

        if num_fields == 1 {
            Err(Error::RepoConfig("missing slash delimiter between fields"))
        } else if num_fields > 2 {
            Err(Error::RepoConfig("more than two fields"))
        } else if num_fields != 2 {
            unreachable!("invalid number of fields");
        } else {
            Ok(Repo(fields[0].to_string(), fields[1].to_string()))
        }
    }
}

impl From<Repo> for crate::Repo {
    fn from(repo: Repo) -> Self {
        crate::Repo::new(repo.0, repo.1)
    }
}

struct RegistryConfigVisitor(PhantomData<fn() -> RegistryConfig>);

impl RegistryConfigVisitor {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de> de::Visitor<'de> for RegistryConfigVisitor {
    type Value = RegistryConfig;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map of registry config data")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        const VALID_FIELDS: &[&str] = &["provider", "oauth_token", "repos"];
        const DEFAULT_PROVIDER: &str = "github";

        let mut provider = None::<&str>;
        let mut oauth_token = None::<String>;
        let mut repos = None::<Vec<&str>>;

        while let Some(key) = map.next_key()? {
            match key {
                "provider" => {
                    provider = Some(map.next_value()?);
                }
                "oauth_token" => {
                    let mut val = map.next_value()?;
                    env::replace_vars(&mut val).map_err(de::Error::custom)?;
                    oauth_token = Some(val);
                }
                "repos" => {
                    repos = Some(map.next_value()?);
                }
                unknown => {
                    return Err(de::Error::unknown_field(unknown, VALID_FIELDS));
                }
            }
        }

        match provider.unwrap_or_else(|| DEFAULT_PROVIDER) {
            "github" => {
                let oauth_token =
                    oauth_token.ok_or_else(|| de::Error::missing_field("oauth_token"))?;
                let repos = repos
                    .ok_or_else(|| de::Error::missing_field("repos"))?
                    .into_iter()
                    .map(|repo| repo.parse())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(de::Error::custom)?;

                Ok(RegistryConfig::GitHub { oauth_token, repos })
            }
            unexpected => Err(de::Error::invalid_value(
                de::Unexpected::Str(unexpected),
                &"a valid provider type",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for RegistryConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(RegistryConfigVisitor::new())
    }
}

/// Returns the default bind address for the server.
fn default_bind_addr() -> SocketAddr {
    SocketAddr::new(net::IpAddr::V4(net::Ipv4Addr::new(0, 0, 0, 0)), 8000)
}

/// Deserialize into a `SocketAddr` by first replacing any environment variables.
fn de_bind_addr<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s = String::deserialize(deserializer)?;
    env::replace_vars(&mut s).map_err(de::Error::custom)?;
    SocketAddr::deserialize(s.into_deserializer())
}
